use sqlrustgo_storage::{FileStorage, StorageEngine, TableInfo, Value};
fn main() {
    {
        let dir=tempfile::tempdir().unwrap();
        let storage=FileStorage::new(dir.path().into()).unwrap();
        let mut e=sqlrustgo::ExecutionEngine::new(std::sync::Arc::new(parking_lot::RwLock::new(storage)));
        for sql in ["CREATE TABLE t (id INT PRIMARY KEY)","INSERT INTO t VALUES (1)","BEGIN","DELETE FROM t WHERE id=1","ROLLBACK"] {
            let r=e.execute(sql);
            println!("HARA historical_sql statement={sql:?} result={:?}",r.as_ref().map(|_| ()));
            if r.is_err() { break; }
        }
        println!("HARA historical_sql expected=[[1]] actual={:?}",e.execute("SELECT * FROM t").map(|r|r.rows));
    }
    for flush in [false,true] {
        let dir=tempfile::tempdir().unwrap();
        let mut s=FileStorage::new_with_buffer_config(dir.path().into(),usize::MAX,true).unwrap();
        let info:TableInfo=serde_json::from_str(r#"{"name":"t","columns":[{"name":"id","data_type":"INTEGER","nullable":false,"primary_key":false}]}"#).unwrap();
        s.create_table(&info).unwrap();
        let begin=s.begin_transaction();
        if let Err(e)=begin { println!("HARA historical begin=UNSUPPORTED {e:?}"); continue; }
        s.insert("t",vec![vec![Value::Integer(42)]]).unwrap();
        if flush { StorageEngine::flush(&mut s).unwrap(); }
        let rollback=s.rollback_transaction();
        println!("HARA historical flush={flush} rollback={rollback:?} expected=0 actual={}",s.scan("t").unwrap().len());
    }
}
