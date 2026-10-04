//! #4986 — GC 不得回收未提交（pending）版本。
//!
//! `VersionedTable::gc` 的两处回收判断此前都只看 `visible_from_ts`，
//! **没有一处检查 `committed`**。而 #4982 让未提交版本在**写入时**就带上
//! `visible_from_ts`，只有 `commit_tx` 才把它改写为提交时刻。
//!
//! 于是「事务开始写入 → 停顿超过 `gc_lag` → GC 跑过 → 再提交」这条
//! 路径上，GC 会把该事务自己的在途写入当作陈旧版本回收。`commit_tx`
//! 随后遍历版本链已找不到它，**写入静默消失，全程 errors = 0**。
//!
//! 现有测试抓不到，是因为它们都在毫秒内跑完，事务持续时间远小于
//! `gc_lag`（生产默认 `MVCC_GC_LAG = 1024`）。

use sqlrustgo_storage::mvcc::VersionedTable;
use sqlrustgo_types::Value;

fn int(i: i64) -> Value {
    Value::Integer(i)
}

/// 场景 A：单版本链。事务写入 → GC 跑过 → 提交。
///
/// 单版本链是 Step 2（整链 evict）命中，最直接的数据丢失路径。
#[test]
fn gc_keeps_pending_version_in_single_version_chain() {
    let t = VersionedTable::new();

    // 事务 7 在 ts=10 写入（tx_id != 0 → committed=false）
    t.put(
        int(1),
        vec![int(1), Value::Text("uncommitted".into())],
        10,
        7,
    );
    assert_eq!(t.scan_visible(1000, 7).len(), 1, "写入后对自己的事务应可见");

    // 推进快照，让 cutoff 越过 pending 版本的 ts
    for _ in 0..1000 {
        t.next_snapshot_ts();
    }
    let dropped = t.gc(1000, 100);
    println!(
        "PROBE single_chain gc_dropped={dropped} still_visible_to_own_tx={}",
        t.scan_visible(1000, 7).len()
    );
    assert_eq!(dropped, 0, "GC 回收了未提交版本 —— #4986 未修复");
    assert_eq!(
        t.scan_visible(1000, 7).len(),
        1,
        "未提交版本在 GC 后消失 —— 该事务提交时写会丢"
    );

    // 提交后数据仍在，且对外可见
    t.commit_tx(7, 1001);
    assert_eq!(
        t.scan_visible(1002, 0).len(),
        1,
        "事务提交后数据丢失（#4986 的实际危害）"
    );
}

/// 场景 B：多版本链。同一 PK 先有一次已提交写入，再有一次未提交写入。
///
/// 命中 Step 1（trim）。pending 版本在链尾，若被 trim 掉，`commit_tx`
/// 找不到可提升的版本，这次更新就静默丢失，读者会继续看到旧值。
#[test]
fn gc_keeps_pending_version_in_multi_version_chain() {
    let t = VersionedTable::new();

    // 事务 1 已提交（旧值）
    t.put(int(1), vec![int(1), Value::Text("v1".into())], 10, 0);
    // 事务 7 未提交（新值）
    t.put(
        int(1),
        vec![int(1), Value::Text("v2-uncommitted".into())],
        20,
        7,
    );

    assert_eq!(t.scan_visible(1000, 7).len(), 1, "链应有一个可见版本");

    for _ in 0..1000 {
        t.next_snapshot_ts();
    }
    let dropped = t.gc(1000, 100);
    println!(
        "PROBE multi_chain gc_dropped={dropped} own_tx_sees={:?}",
        t.scan_visible(1000, 7)
    );
    // 注意：这里 dropped **不是** 0，而且不该是 0。链是
    // [v1(ts=10, committed), v2(ts=20, pending)]，回收 v1 正是 GC 的本职
    // （场景 C 单独断言了这一点）。要检查的是 v2 是否幸存 ——
    // 若修复前 Step 1 会把 v2 也 trim 掉，提交后就会「复活」成陈旧的 v1。
    assert_eq!(
        dropped, 1,
        "只应回收陈旧的已提交版本 v1，pending 的 v2 必须留下"
    );

    // 自己的事务仍应看到自己的未提交值
    let own = t.scan_visible(1000, 7);
    assert_eq!(own.len(), 1);
    assert_eq!(
        own[0].1[1],
        Value::Text("v2-uncommitted".into()),
        "自己的未提交写入被 GC 吃掉了"
    );

    // 提交后应看到新值
    t.commit_tx(7, 1001);
    let after = t.scan_visible(1002, 0);
    assert_eq!(after.len(), 1);
    assert_eq!(
        after[0].1[1],
        Value::Text("v2-uncommitted".into()),
        "提交后应看到新值"
    );
}

/// 场景 C（反向约束）：真正陈旧的**已提交**版本仍必须被回收。
///
/// 没有这一条，把 `committed` 判断加上就可能变成「GC 从此什么都不做」。
/// 那是用关掉 GC 来假装修复问题。
#[test]
fn gc_still_reclaims_stale_committed_versions() {
    let t = VersionedTable::new();

    t.put(int(1), vec![int(1), Value::Text("old".into())], 10, 0);
    t.put(int(2), vec![int(2), Value::Text("old".into())], 10, 0);
    t.put(int(3), vec![int(3), Value::Text("old".into())], 10, 0);

    for _ in 0..1000 {
        t.next_snapshot_ts();
    }
    let dropped = t.gc(1000, 100);
    let remaining = t.scan_visible(1002, 0).len();
    println!("PROBE stale_committed gc_dropped={dropped} remaining={remaining}");
    assert_eq!(dropped, 3, "已提交的陈旧版本没有被回收 —— GC 被改瘫了");
    assert_eq!(remaining, 0);
}

/// 场景 D：pending 版本在事务结束后能被正常回收。
///
/// 跳过 pending **不等于**永久泄漏：`commit_tx` 把 `visible_from_ts`
/// 改写成提交时刻（当前值），下一轮 GC 就正常回收它。
#[test]
fn pending_version_becomes_reclaimable_after_commit() {
    let t = VersionedTable::new();
    t.put(int(1), vec![int(1), Value::Text("x".into())], 10, 7);

    for _ in 0..1000 {
        t.next_snapshot_ts();
    }
    assert_eq!(t.gc(1000, 100), 0, "pending 不该被回收");

    // 事务提交：visible_from_ts 被改写为 1001（当前时刻）
    t.commit_tx(7, 1001);

    // 再推进足够久，它就该变成陈旧版本并被回收
    for _ in 0..2000 {
        t.next_snapshot_ts();
    }
    let dropped = t.gc(3001, 100);
    let remaining = t.scan_visible(3002, 0).len();
    println!("PROBE after_commit gc_dropped={dropped} remaining={remaining}");
    assert_eq!(dropped, 1, "提交后该版本未被回收 —— pending 泄漏了");
    assert_eq!(remaining, 0);
}

/// 场景 E：pending 版本位于**链中间** —— 这才是 Step 1 唯一能碰到的位置。
///
/// `gc` 的 Step 1 循环条件是 `while i + 1 < chain.len()`，即**永远不检查
/// 最后一个版本**。所以场景 A/B 里 pending 在链尾，Step 1 根本够不着它，
/// 去掉 Step 1 的 `committed` 判断也照样全绿 —— 这个盲区是实测发现的。
///
/// 真实的触发路径：事务 A 更新某行（pending），事务 B 随后又更新同一行
/// （已提交）。链变成 [旧, A的pending, B的已提交]，A 的 pending 落在中间。
/// GC 一旦 trim 掉它，A 再 COMMIT 就找不到可提升的版本，这次更新静默丢失，
/// 读者会继续看到旧值。
#[test]
fn gc_keeps_pending_version_sandwiched_in_chain_middle() {
    let t = VersionedTable::new();

    t.put(int(1), vec![int(1), Value::Text("v1".into())], 10, 0); // 已提交旧值
    t.put(
        int(1),
        vec![int(1), Value::Text("v2-A-pending".into())],
        20,
        7,
    ); // A 未提交
    t.put(
        int(1),
        vec![int(1), Value::Text("v3-B-committed".into())],
        30,
        0,
    ); // B 已提交

    for _ in 0..1000 {
        t.next_snapshot_ts();
    }
    let dropped = t.gc(1000, 100);
    println!(
        "PROBE sandwich gc_dropped={dropped} own_tx_sees={:?}",
        t.scan_visible(1000, 7)
    );

    // 中间版本是否幸存，不能靠 `scan_visible` 观察：`find_visible` 是
    // `iter().rev().find(..)`，从**链尾**开始找。v3 在链尾且已提交，
    // 所以无论 v2 是否还在，读者都看到 v3。而且 `commit_tx` 只改写
    // `visible_from_ts`、**不重排版本链**，所以「最后提交」也不等于
    // 「链尾」。
    //
    // 改用可观察的判据：提交后再推一轮 GC。
    //   - v2 幸存 → 提交时 committed=true 且 ts 被改写为 1001，
    //     推进到 cutoff 越过 1001 后它会被正常回收，dropped 增 1。
    //   - v2 已被 trim → 链里根本没有它，dropped 不变。
    t.commit_tx(7, 1001);
    for _ in 0..2000 {
        t.next_snapshot_ts();
    }
    let dropped_after_commit = t.gc(3001, 100);
    println!("PROBE sandwich_after_commit gc_dropped={dropped_after_commit}");
    assert_eq!(
        dropped_after_commit, 2,
        "链中间的未提交版本没有幸存到提交之后 —— A 的更新在提交时丢失。\n\
         (v1 陈旧 + v2-A-pending 提交后变陈旧 = 2；只有 1 说明 v2 已被 trim)"
    );
}
