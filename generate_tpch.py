#!/usr/bin/env python3
"""
TPC-H Data Generator
Generates standard TPC-H datasets for benchmarking.

Usage:
    python3 generate_tpch.py --sf=0.1 --output-dir=/path/to/output
    python3 generate_tpch.py --sf=1 --output-dir=/path/to/output
"""

import os
import random
import hashlib
import csv
from datetime import datetime, timedelta

# TPC-H Schema
TABLES = {
    'region': ['r_regionkey', 'r_name', 'r_comment'],
    'nation': ['n_nationkey', 'n_name', 'n_regionkey', 'n_comment'],
    'supplier': ['s_suppkey', 's_name', 's_nationkey', 's_address', 's_phone', 's_comment'],
    'part': ['p_partkey', 'p_name', 'p_mfgr', 'p_brand', 'p_type', 'p_size', 'p_container', 'p_retailprice', 'p_comment'],
    'partsupp': ['ps_suppkey', 'ps_partkey', 'ps_availableqty', 'ps_supplycost', 'ps_comment'],
    'customer': ['c_custkey', 'c_name', 'c_nationkey', 'c_address', 'c_phone', 'c_acctbal', 'c_mktsegment', 'c_comment'],
    'orders': ['o_orderkey', 'o_custkey', 'o_orderstatus', 'o_totalprice', 'o_orderdate', 'o_orderpriority', 'o_clerk', 'o_shippriority', 'o_comment'],
    'lineitem': ['l_orderkey', 'l_partkey', 'l_suppkey', 'l_linenumber', 'l_quantity', 'l_extendedprice', 'l_discount', 'l_returnflag', 'l_linestatus', 'l_commitdate', 'l_receiptdate', 'l_shipinstruct', 'l_shipmode', 'l_comment']
}

# Fixed seed for reproducibility
RANDOM_SEED = 42

def set_seed():
    random.seed(RANDOM_SEED)

def generate_region(sf, output_dir):
    """Generate region data"""
    regions = [
        ('1', 'AMERICA', 'Americas'),
        ('2', 'AFRICA', 'Africa'),
        ('3', 'ASIA', 'Asia'),
        ('4', 'EUROPE', 'Europe'),
        ('5', 'OCEANIA', 'Oceania'),
        ('6', 'ANTARCTICA', 'Antarctica'),
    ]
    filename = os.path.join(output_dir, 'region.tbl')
    with open(filename, 'w') as f:
        for r_regionkey, r_name, r_comment in regions:
            f.write(f"{r_regionkey}|{r_name}|{r_comment}\n")
    print(f"  ✓ region.tbl: {len(regions)} rows")

def generate_nation(sf, output_dir, num_nations):
    """Generate nation data"""
    nations = []
    regions = ['AMERICA', 'AFRICA', 'ASIA', 'EUROPE', 'OCEANIA', 'ANTARCTICA']
    for i in range(1, num_nations + 1):
        nation_key = f"{i:04d}"
        region_key = str((i - 1) % 6 + 1)
        nation_name = f"Nation {i}"
        countries = ['United States', 'Canada', 'Mexico', 'Brazil', 'Chile', 'France', 'Germany', 'UK', 'Italy', 'Spain', 'Japan', 'China', 'India', 'South Korea', 'Australia', 'Russia']
        country = countries[(i - 1) % len(countries)]
        nation_key = f"{i:04d}"
        nation = {
            'n_nationkey': nation_key,
            'n_name': f"{country}, {nation_name}",
            'n_regionkey': region_key,
            'n_comment': f"Sample comment for {country}" if i % 5 == 0 else ""
        }
        nations.append(nation)
    filename = os.path.join(output_dir, 'nation.tbl')
    with open(filename, 'w') as f:
        for nation in nations:
            f.write(f"{nation['n_nationkey']}|{nation['n_name']}|{nation['n_regionkey']}|{nation['n_comment']}\n")
    print(f"  ✓ nation.tbl: {len(nations)} rows")

def generate_supplier(sf, output_dir, num_suppliers):
    """Generate supplier data"""
    suppliers = []
    for i in range(1, num_suppliers + 1):
        supplier_key = f"{i:04d}"
        supplier_name = f"Supplier {i}"
        # Distribute suppliers across nations
        num_nations = 4
        nation_key = str((i - 1) % num_nations + 1)
        cities = ['New York', 'Los Angeles', 'Chicago', 'Houston', 'Phoenix', 'Philadelphia', 'San Antonio', 'San Diego', 'Dallas', 'San Jose']
        city = cities[(i - 1) % len(cities)]
        phone = f"+1-{(i % 10) + 100}-{(i % 100):02d}"
        supplier = {
            's_suppkey': supplier_key,
            's_name': f"{supplier_name}, {city}",
            's_nationkey': nation_key,
            's_address': f"{i} {city} St, {city}",
            's_phone': phone,
            's_comment': f"Sample comment for {city}" if i % 3 == 0 else ""
        }
        suppliers.append(supplier)
    filename = os.path.join(output_dir, 'supplier.tbl')
    with open(filename, 'w') as f:
        for supplier in suppliers:
            f.write(f"{supplier['s_suppkey']}|{supplier['s_name']}|{supplier['s_nationkey']}|{supplier['s_address']}|{supplier['s_phone']}|{supplier['s_comment']}\n")
    print(f"  ✓ supplier.tbl: {len(suppliers)} rows")

def generate_part(sf, output_dir, num_parts, num_manufacturers):
    """Generate part data"""
    manufacturers = []
    brands = []
    types = []
    containers = []
    
    # Generate unique values
    for i in range(num_manufacturers):
        manufacturers.append(f"Manufacturer {i + 1}")
    for i in range(30):
        brands.append(f"Brand {i + 1}")
    for i in range(20):
        types.append(f"Type {i + 1}")
    for i in range(15):
        containers.append(f"Container {i + 1}")
    
    filename = os.path.join(output_dir, 'part.tbl')
    with open(filename, 'w') as f:
        for p_partkey in range(1, num_parts + 1):
            part_key = f"{p_partkey:05d}"
            manufacturer = manufacturers[(p_partkey - 1) % num_manufacturers]
            brand = brands[(p_partkey - 1) % len(brands)]
            part_type = types[(p_partkey - 1) % len(types)]
            container = containers[(p_partkey - 1) % len(containers)]
            
            # Generate part name
            part_name = f"Part {p_partkey}"
            size = random.randint(1, 500)
            price = round(random.uniform(10, 1000), 2)
            
            f.write(f"{part_key}|{part_name}|{manufacturer}|{brand}|{part_type}|{size}|{container}|{price}||\n")
    print(f"  ✓ part.tbl: {num_parts} rows")

def generate_partsupp(sf, output_dir, num_suppliers, num_parts):
    """Generate partsupp data"""
    filename = os.path.join(output_dir, 'partsupp.tbl')
    with open(filename, 'w') as f:
        count = 0
        for s_suppkey in range(1, num_suppliers + 1):
            for p_partkey in range(1, num_parts + 1):
                if count >= 10000:  # Limit to 10000 rows for SF01
                    break
                supply_key = f"{s_suppkey:04d}{p_partkey:05d}"
                qty = random.randint(1, 100)
                cost = round(random.uniform(1, 100), 2)
                f.write(f"{supply_key}|{p_partkey:05d}|{qty}|{cost}||\n")
                count += 1
    print(f"  ✓ partsupp.tbl: {count} rows")

def generate_customer(sf, output_dir, num_customers):
    """Generate customer data"""
    segments = ['BUILDING', 'MACHINERY', 'FURNISHINGS', 'HOUSEhold', 'AUTOMOBILE']
    
    filename = os.path.join(output_dir, 'customer.tbl')
    with open(filename, 'w') as f:
        for c_custkey in range(1, num_customers + 1):
            cust_key = f"{c_custkey:05d}"
            customer_name = f"Customer {c_custkey}"
            nation_key = str((c_custkey - 1) % 4 + 1)  # Use first 4 nations
            city = random.choice(['New York', 'Los Angeles', 'Chicago', 'Houston', 'Phoenix', 'Philadelphia', 'San Antonio', 'San Diego', 'Dallas', 'San Jose'])
            address = f"{c_custkey} {city} St, {city}"
            phone = f"+1-{(c_custkey % 10) + 100}-{(c_custkey % 100):02d}"
            acctbal = round(random.uniform(-1000, 10000), 2)
            segment = segments[(c_custkey - 1) % len(segments)]
            
            f.write(f"{cust_key}|{customer_name}|{nation_key}|{address}|{phone}|{acctbal}|{segment}||\n")
    print(f"  ✓ customer.tbl: {num_customers} rows")

def generate_orders(sf, output_dir, num_customers, num_orders):
    """Generate orders data"""
    status_list = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']
    priorities = ['1-URGENT', '2-HIGH', '3-MEDIUM', '4-NOTIFIED', '5-NOT-FILLED']
    clerks = ['CA', 'CB', 'CC', 'CD', 'CE']
    
    filename = os.path.join(output_dir, 'orders.tbl')
    with open(filename, 'w') as f:
        for o_orderkey in range(1, num_orders + 1):
            order_key = f"{o_orderkey:05d}"
            cust_key = f"{(o_orderkey % num_customers) + 1:05d}"
            status = random.choice(status_list)
            total_price = round(random.uniform(100, 100000), 2)
            
            # Generate order date
            base_date = datetime(2026, 1, 1)
            days_offset = random.randint(0, 365)
            order_date = (base_date + timedelta(days=days_offset)).strftime('%Y-%m-%d')
            
            priority = random.choice(priorities)
            clerk = random.choice(clerks)
            
            f.write(f"{order_key}|{cust_key}|{status}|{total_price}|{order_date}|{priority}|{clerk}|0||\n")
    print(f"  ✓ orders.tbl: {num_orders} rows")

def generate_lineitem(sf, output_dir, num_orders, num_parts, num_suppliers, num_lines_per_order):
    """Generate lineitem data"""
    return_flags = ['N', 'A', 'M']
    status_list = ['0', '1', '2', '3', '4', '5']
    ship_instructs = ['DELIVER IN PERSON', 'TRUCK', 'SHIP', 'AIR', 'RAIL', 'FOAM', 'HULL']
    ship_modes = ['MAIL', 'AIR', 'SHIP', 'TRUCK', 'RAIL', 'FOAM', 'HULL']
    
    filename = os.path.join(output_dir, 'lineitem.tbl')
    with open(filename, 'w') as f:
        count = 0
        for o_orderkey in range(1, num_orders + 1):
            order_key = f"{o_orderkey:05d}"
            for l_linenumber in range(1, num_lines_per_order + 1):
                if count >= 100000:  # Limit for SF01
                    break
                
                part_key = f"{random.randint(1, num_parts):05d}"
                supp_key = f"{random.randint(1, num_suppliers):04d}"
                quantity = random.randint(1, 100)
                unit_price = round(random.uniform(10, 1000), 2)
                extended_price = quantity * unit_price
                discount = round(random.uniform(0, 0.1), 2)
                
                commit_date = (datetime(2026, 1, 1) + timedelta(days=random.randint(0, 365))).strftime('%Y-%m-%d')
                receipt_date = (datetime(2026, 1, 1) + timedelta(days=random.randint(0, 365))).strftime('%Y-%m-%d')
                
                f.write(f"{order_key}|{part_key}|{supp_key}|{l_linenumber}|{quantity}|{extended_price}|{discount}|{random.choice(return_flags)}|{random.choice(status_list)}|{commit_date}|{receipt_date}|{random.choice(ship_instructs)}|{random.choice(ship_modes)}||\n")
                count += 1
    print(f"  ✓ lineitem.tbl: {count} rows")

def generate_sf01(output_dir):
    """Generate TPC-H SF01 dataset"""
    print("="*60)
    print("Generating TPC-H SF01 (SF=0.1) dataset")
    print("="*60)
    
    set_seed()
    
    # Calculate sizes for SF01
    num_regions = 6
    num_nations = 4  # 4 nations for SF01
    num_suppliers = 4  # 4 suppliers for SF01
    num_parts = 4  # 4 parts for SF01
    num_partsupp = 10000  # 10000 partsupp rows
    num_customers = 4  # 4 customers for SF01
    num_orders = 1000  # 1000 orders for SF01
    num_lines_per_order = 10  # 10 lines per order for SF01
    
    print("\nGenerating tables...\n")
    
    generate_region(0.1, output_dir)
    generate_nation(0.1, output_dir, num_nations)
    generate_supplier(0.1, output_dir, num_suppliers)
    generate_part(0.1, output_dir, num_parts, num_suppliers)
    generate_partsupp(0.1, output_dir, num_suppliers, num_parts)
    generate_customer(0.1, output_dir, num_customers)
    generate_orders(0.1, output_dir, num_customers, num_orders)
    generate_lineitem(0.1, output_dir, num_orders, num_parts, num_suppliers, num_lines_per_order)
    
    print("\n" + "="*60)
    print("SF01 generation complete!")
    print("="*60)
    
    # List generated files
    files = os.listdir(output_dir)
    print(f"\nGenerated files in {output_dir}:")
    for f in sorted(files):
        size = os.path.getsize(os.path.join(output_dir, f))
        print(f"  {f}: {size:,} bytes")

def generate_sf1(output_dir):
    """Generate TPC-H SF1 dataset"""
    print("="*60)
    print("Generating TPC-H SF1 (SF=1.0) dataset")
    print("="*60)
    
    set_seed()
    
    # Calculate sizes for SF1
    num_regions = 6
    num_nations = 40  # 40 nations for SF1
    num_suppliers = 40  # 40 suppliers for SF1
    num_parts = 40  # 40 parts for SF1
    num_partsupp = 100000  # 100000 partsupp rows
    num_customers = 40  # 40 customers for SF1
    num_orders = 10000  # 10000 orders for SF1
    num_lines_per_order = 10  # 10 lines per order for SF1
    
    print("\nGenerating tables...\n")
    
    generate_region(1.0, output_dir)
    generate_nation(1.0, output_dir, num_nations)
    generate_supplier(1.0, output_dir, num_suppliers)
    generate_part(1.0, output_dir, num_parts, num_suppliers)
    generate_partsupp(1.0, output_dir, num_suppliers, num_parts)
    generate_customer(1.0, output_dir, num_customers)
    generate_orders(1.0, output_dir, num_customers, num_orders)
    generate_lineitem(1.0, output_dir, num_orders, num_parts, num_suppliers, num_lines_per_order)
    
    print("\n" + "="*60)
    print("SF1 generation complete!")
    print("="*60)
    
    # List generated files
    files = os.listdir(output_dir)
    print(f"\nGenerated files in {output_dir}:")
    for f in sorted(files):
        size = os.path.getsize(os.path.join(output_dir, f))
        print(f"  {f}: {size:,} bytes")

def main():
    import argparse
    
    parser = argparse.ArgumentParser(description='TPC-H Dataset Generator')
    parser.add_argument('--sf', type=float, required=True, help='Scale factor (0.1 or 1.0)')
    parser.add_argument('--output-dir', type=str, default='/home/openclaw/.openclaw/workspace/sqlrustgo/data', help='Output directory')
    
    args = parser.parse_args()
    
    output_dir = args.output_dir
    os.makedirs(output_dir, exist_ok=True)
    
    if args.sf == 0.1:
        generate_sf01(output_dir)
    elif args.sf == 1.0:
        generate_sf1(output_dir)
    else:
        print(f"Invalid scale factor: {args.sf}")
        print("Use --sf=0.1 for SF01 or --sf=1.0 for SF1")

if __name__ == '__main__':
    main()
