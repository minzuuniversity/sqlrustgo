#!/usr/bin/env python3
"""Parameterized INSERT/UPDATE/DELETE templates for TPC-H mixed-workload SOAK."""
import random
import threading
from typing import Optional


MIN_ROWS = {
    "orders": 10000,
    "lineitem": 50000,
    "customer": 1000,
    "partsupp": 50000,
    "part": 1500,
}

PK_COLUMN = {
    "orders": "o_orderkey",
    "lineitem": None,
    "customer": "c_custkey",
    "partsupp": None,
    "part": "p_partkey",
}


class IdPool:
    def __init__(self, table: str, worker_id: int, block_size: int = 1000,
                 base_offset: int = 100_000_000):
        self.table = table
        self.worker_id = worker_id
        self.block_size = block_size
        self.base_offset = base_offset
        self.current = base_offset + worker_id * block_size
        self.high_water = self.current + block_size
        self.lock = threading.Lock()

    def next(self) -> int:
        with self.lock:
            if self.current >= self.high_water:
                raise PoolExhausted(self.table, self.worker_id)
            val = self.current
            self.current += 1
            return val

    def refresh(self, db_max: int):
        with self.lock:
            self.current = max(db_max + 1, self.base_offset)
            self.high_water = self.current + self.block_size

    def reserve(self) -> int:
        with self.lock:
            return self.current


class PoolExhausted(Exception):
    def __init__(self, table: str, worker_id: int):
        self.table = table
        self.worker_id = worker_id
        super().__init__(f"ID pool exhausted for {table} worker {worker_id}")


def _q(s: Optional[str]) -> str:
    if s is None:
        return "''"
    return "'" + s.replace("'", "''") + "'"


def insert_order(oid: int, custkey: int, totalprice: float,
                 orderdate: str, priority: str = "1-URGENT",
                 clerk: str = "Clerk#000000001", shippriority: int = 0) -> str:
    return (
        f"INSERT INTO orders VALUES ("
        f"{oid}, {custkey}, 'O', {totalprice:.2f}, '{orderdate}', "
        f"{_q(priority)}, {_q(clerk)}, {shippriority}, 'auto-generated');"
    )


def update_order_status(orderkey: int, new_status: str = "F") -> str:
    return (
        f"UPDATE orders SET o_orderstatus = '{new_status}' "
        f"WHERE o_orderkey = {orderkey} AND o_orderstatus = 'O';"
    )


def delete_old_orders(cutoff_date: str = "2015-01-01") -> str:
    return (
        f"DELETE FROM orders WHERE o_orderdate < '{cutoff_date}' "
        f"AND o_orderstatus IN ('C', 'X') LIMIT 100;"
    )


def insert_lineitem(orderkey: int, partkey: int, suppkey: int, linenumber: int,
                    quantity: int, extendedprice: float, discount: float = 0.05,
                    tax: float = 0.0, shipdate: str = "1995-01-01",
                    commitdate: str = "1995-01-01",
                    receiptdate: str = "1995-01-01") -> str:
    return (
        f"INSERT INTO lineitem VALUES ("
        f"{orderkey}, {partkey}, {suppkey}, {linenumber}, {quantity}, "
        f"{extendedprice:.2f}, {discount:.2f}, {tax:.2f}, "
        f"'R', 'O', '{shipdate}', '{commitdate}', '{receiptdate}', "
        f"'DELIVERED IN PERSON', 'TRUCK', 'auto-generated');"
    )


def update_lineitem_qty(orderkey: int, linenumber: int, delta: int = 1) -> str:
    return (
        f"UPDATE lineitem SET l_quantity = l_quantity + {delta} "
        f"WHERE l_orderkey = {orderkey} AND l_linenumber = {linenumber};"
    )


def delete_old_lineitems(cutoff_date: str = "2015-01-01") -> str:
    return (
        f"DELETE FROM lineitem WHERE l_orderkey IN ("
        f"  SELECT o_orderkey FROM orders "
        f"  WHERE o_orderdate < '{cutoff_date}' LIMIT 100);"
    )


def insert_customer(custkey: int, nationkey: int, acctbal: float,
                    mktsegment: str = "AUTOMOBILE",
                    name: str = "Customer#000001") -> str:
    return (
        f"INSERT INTO customer VALUES ("
        f"{custkey}, {_q(name)}, 'auto-generated address', {nationkey}, "
        f"'800-000-0000', {acctbal:.2f}, {_q(mktsegment)}, 'auto-comment');"
    )


def update_customer_balance(custkey: int, delta: float = 10.0) -> str:
    return (
        f"UPDATE customer SET c_acctbal = c_acctbal + {delta:.2f} "
        f"WHERE c_custkey = {custkey};"
    )


def delete_inactive_customers(nationkey: int) -> str:
    return (
        f"DELETE FROM customer WHERE c_acctbal < -900 "
        f"AND c_nationkey = {nationkey} LIMIT 50;"
    )


def insert_part(partkey: int, size: int = 10, retailprice: float = 100.0,
                name: str = "part name", mfgr: str = "Manufacturer#1",
                brand: str = "Brand#11", ptype: str = "STANDARD POLISHED TIN",
                container: str = "MED BOX") -> str:
    return (
        f"INSERT INTO part VALUES ("
        f"{partkey}, {_q(name)}, {_q(mfgr)}, {_q(brand)}, {_q(ptype)}, "
        f"{size}, {_q(container)}, {retailprice:.2f}, 'auto-comment');"
    )


def update_part_price(partkey: int, multiplier: float = 1.01) -> str:
    return (
        f"UPDATE part SET p_retailprice = p_retailprice * {multiplier:.4f} "
        f"WHERE p_partkey = {partkey};"
    )


def delete_obsolete_parts(threshold_price: float = 900.0) -> str:
    return (
        f"DELETE FROM part WHERE p_retailprice < {threshold_price:.2f} "
        f"AND p_size > 30 LIMIT 20;"
    )


def insert_partsupp(partkey: int, suppkey: int, availqty: int = 100,
                    supplycost: float = 50.0) -> str:
    return (
        f"INSERT INTO partsupp VALUES ("
        f"{partkey}, {suppkey}, {availqty}, {supplycost:.2f}, 'auto-comment');"
    )


def update_partsupp_cost(partkey: int, suppkey: int, multiplier: float = 1.05) -> str:
    return (
        f"UPDATE partsupp SET ps_supplycost = ps_supplycost * {multiplier:.4f} "
        f"WHERE ps_partkey = {partkey} AND ps_suppkey = {suppkey};"
    )


def delete_orphan_partsupp() -> str:
    return "DELETE FROM partsupp WHERE ps_availqty = 0 LIMIT 50;"


def random_crud_for_table(table: str, id_pool: IdPool, rng: random.Random,
                          ctx: dict) -> str:
    op = rng.choice(["insert", "update", "delete"])

    if table == "orders":
        if op == "insert":
            new_id = id_pool.next()
            sql = insert_order(new_id, ctx["custkey"], rng.uniform(100, 5000),
                               ctx.get("orderdate", "1995-06-01"))
            ctx["last_orderkey"] = new_id
            return sql
        elif op == "update":
            ok = ctx.get("last_orderkey", id_pool.reserve())
            return update_order_status(ok, rng.choice(["F", "C", "O"]))
        return delete_old_orders()

    if table == "lineitem":
        if op == "insert":
            ok = ctx.get("last_orderkey", id_pool.reserve())
            sql = insert_lineitem(ok, ctx["partkey"], ctx["suppkey"],
                                  ctx.get("linenumber", 1),
                                  rng.randint(1, 50), rng.uniform(10, 1000))
            ctx["linenumber"] = ctx.get("linenumber", 1) + 1
            return sql
        elif op == "update":
            ok = ctx.get("last_orderkey", id_pool.reserve())
            return update_lineitem_qty(ok, 1, rng.randint(1, 5))
        return delete_old_lineitems()

    if table == "customer":
        if op == "insert":
            new_id = id_pool.next()
            sql = insert_customer(new_id, ctx["nationkey"],
                                  rng.uniform(-1000, 10000))
            ctx["custkey"] = new_id
            return sql
        elif op == "update":
            return update_customer_balance(
                ctx.get("custkey", id_pool.reserve()),
                rng.uniform(-50, 100))
        return delete_inactive_customers(ctx["nationkey"])

    if table == "part":
        if op == "insert":
            new_id = id_pool.next()
            sql = insert_part(new_id, rng.randint(1, 50), rng.uniform(50, 2000))
            ctx["partkey"] = new_id
            return sql
        elif op == "update":
            return update_part_price(
                ctx.get("partkey", id_pool.reserve()),
                rng.uniform(0.95, 1.10))
        return delete_obsolete_parts()

    if table == "partsupp":
        if op == "insert":
            return insert_partsupp(ctx["partkey"], ctx["suppkey"],
                                  rng.randint(0, 1000), rng.uniform(10, 500))
        elif op == "update":
            return update_partsupp_cost(ctx["partkey"], ctx["suppkey"],
                                        rng.uniform(0.95, 1.10))
        return delete_orphan_partsupp()

    raise ValueError(f"Unknown table: {table}")


def validate_fk(conn_func, sql: str) -> bool:
    try:
        conn_func(sql)
        return True
    except Exception:
        return False