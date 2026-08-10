## 1. Update #3887 issue body

- [x] 1.1 GET current #3887 body
- [x] 1.2 构造新 body: #3900 / #3906 从 `- [ ]` 改为 `- [x]`, 添加 7 字段 evidence block
- [x] 1.3 PATCH /issues/3887
- [x] 1.4 验证 GET 显示新勾选状态

## 2. Post #3900 final evidence comment

- [x] 2.1 Post comment to #3900: codex #88732 reopen 整改完成
- [x] 2.2 含 7 字段 evidence + C-ARCH-05 follow-up 说明
- [x] 2.3 Cross-reference: #3887 勾选已更新

## 3. Re-apply close #3900

- [x] 3.1 PATCH /issues/3900 state=closed
- [x] 3.2 验证 closed 状态

## 4. Post #3906 final evidence comment

- [x] 4.1 Post comment to #3906: 勾选状态同步
- [x] 4.2 含 7 字段 evidence + R2.4/6/7/8 follow-up 跟踪
- [x] 4.3 Cross-reference: #3887 勾选已更新

## 5. Verify

- [x] 5.1 GET /issues/3887: body 中 #3900 / #3906 都是 `[x]`
- [x] 5.2 GET /issues/3900: state=closed
- [x] 5.3 GET /issues/3906: state=closed
