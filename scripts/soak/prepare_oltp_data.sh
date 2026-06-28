#!/usr/bin/env bash
# prepare_oltp_data.sh — Seed OLTP SOAK test data
# Usage:
#   bash scripts/soak/prepare_oltp_data.sh --host=127.0.0.1 --port=3396 --user=root
#   bash scripts/soak/prepare_oltp_data.sh --level=small --host=127.0.0.1 --port=3396
#
# Levels:
#   small   — 1000 customers, 5000 orders, 20000 items
#   medium  — 10000 customers, 50000 orders, 200000 items
#   large   — 50000 customers, 200000 orders, 800000 items
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

# Defaults
HOST="127.0.0.1"
PORT=3396
USER="root"
PASSWORD=""
LEVEL="medium"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --host=*)    HOST="${1#*=}"; shift ;;
        --host)      HOST="$2"; shift 2 ;;
        --port=*)    PORT="${1#*=}"; shift ;;
        --port)      PORT="$2"; shift 2 ;;
        --user=*)    USER="${1#*=}"; shift ;;
        --user)      USER="$2"; shift 2 ;;
        --password=*) PASSWORD="${1#*=}"; shift ;;
        --password)  PASSWORD="$2"; shift 2 ;;
        --level=*)   LEVEL="${1#*=}"; shift ;;
        --level)     LEVEL="$2"; shift 2 ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# Row counts by level
case "$LEVEL" in
    small)
        CUSTOMER_ROWS=1000
        ORDER_ROWS=5000
        ITEM_ROWS=20000
        BATCH_SIZE=100
        ;;
    medium)
        CUSTOMER_ROWS=10000
        ORDER_ROWS=50000
        ITEM_ROWS=200000
        BATCH_SIZE=100
        ;;
    large)
        CUSTOMER_ROWS=50000
        ORDER_ROWS=200000
        ITEM_ROWS=800000
        BATCH_SIZE=200
        ;;
    *) echo "ERROR: --level must be small|medium|large, got '$LEVEL'"; exit 1 ;;
esac

echo "=== OLTP Data Seeder ==="
echo "  Level:        $LEVEL"
echo "  Customers:    $CUSTOMER_ROWS"
echo "  Orders:       $ORDER_ROWS"
echo "  Items:        $ITEM_ROWS"
echo "  Batch size:   $BATCH_SIZE"
echo "  Target:       $HOST:$PORT/$USER"
echo ""

# Connect string
CONN_ARGS=("-h" "$HOST" "-P" "$PORT" "-u" "$USER")
[[ -n "$PASSWORD" ]] && CONN_ARGS+=("-p$PASSWORD")

# Create schema
echo "=== Creating schema ==="
mysql "${CONN_ARGS[@]}" < "$SCRIPT_DIR/oltp_schema.sql" 2>/dev/null || true
echo "Schema created."

# Seed data using Python (faster + more reliable than bash)
echo "=== Seeding data ==="
CUSTOMER_ROWS=$CUSTOMER_ROWS \
ORDER_ROWS=$ORDER_ROWS \
ITEM_ROWS=$ITEM_ROWS \
BATCH_SIZE=$BATCH_SIZE \
HOST=$HOST PORT=$PORT USER=$USER PASSWORD=$PASSWORD \
python3 << 'PYEOF'
import os, random, subprocess, time, sys

host = os.environ['HOST']
port = os.environ['PORT']
user = os.environ['USER']
password = os.environ.get('PASSWORD', '')
customer_rows = int(os.environ['CUSTOMER_ROWS'])
order_rows = int(os.environ['ORDER_ROWS'])
item_rows = int(os.environ['ITEM_ROWS'])
batch_size = int(os.environ['BATCH_SIZE'])

def run_sql(sql):
    conn = ['mysql', '-h', host, '-P', port, '-u', user]
    if password: conn.append(f'-p{password}')
    conn.append('-e')
    r = subprocess.run(conn + [sql], capture_output=True, text=True)
    if r.returncode != 0:
        print(f"  SQL error: {r.stderr[:200]}", file=sys.stderr)
        return False
    return True

cities = ['New York', 'Beijing', 'Tokyo', 'London', 'Paris', 'Sydney', 'Berlin', 'Toronto', 'Mumbai', 'Seoul']
regions = ['NA', 'APAC', 'EU']
statuses = ['pending', 'processing', 'shipped', 'delivered', 'cancelled']
payments = ['credit_card', 'debit_card', 'paypal', 'bank_transfer']
base_ts = int(time.time()) - 86400 * 365

# ── Customers ──
print(f"  Seeding {customer_rows} customers...", flush=True)
batch = []
for i in range(1, customer_rows + 1):
    c_id = i
    name = f'Customer_{c_id}'
    email = f'user{c_id}@example.com'
    phone = f'+1-555-{random.randint(1000,9999):04d}'
    address = f'{random.randint(1,999)} Main St'
    city = cities[c_id % len(cities)]
    region = regions[c_id % len(regions)]
    balance = round(random.uniform(-1000, 5000), 2)
    created = time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(base_ts + random.randint(0, 86400*365)))
    batch.append(f"INSERT INTO customers (c_id, c_name, c_email, c_phone, c_address, c_city, c_region, c_balance, c_created_at, c_updated_at) VALUES ({c_id}, '{name}', '{email}', '{phone}', '{address}', '{city}', '{region}', {balance}, '{created}', '{created}')")
    if len(batch) >= batch_size:
        run_sql(';\n'.join(batch))
        batch = []
if batch: run_sql(';\n'.join(batch))
print(f"  ✓ Customers: {customer_rows}", flush=True)

# ── Orders ──
print(f"  Seeding {order_rows} orders...", flush=True)
batch = []
for i in range(1, order_rows + 1):
    o_id = i
    o_cid = random.randint(1, customer_rows)
    o_status = statuses[i % len(statuses)]
    o_total = round(random.uniform(10, 5000), 2)
    o_discount = round(random.uniform(0, 0.3), 4)
    o_tax = round(o_total * random.uniform(0.05, 0.15), 2)
    o_pay = payments[i % len(payments)]
    created = time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(base_ts + random.randint(0, 86400*365)))
    batch.append(f"INSERT INTO orders (o_id, o_customer_id, o_status, o_total, o_discount, o_tax, o_payment_method, o_created_at, o_updated_at) VALUES ({o_id}, {o_cid}, '{o_status}', {o_total}, {o_discount}, {o_tax}, '{o_pay}', '{created}', '{created}')")
    if len(batch) >= batch_size:
        run_sql(';\n'.join(batch))
        batch = []
if batch: run_sql(';\n'.join(batch))
print(f"  ✓ Orders: {order_rows}", flush=True)

# ── Order Items ──
print(f"  Seeding {item_rows} order items...", flush=True)
batch = []
for i in range(1, item_rows + 1):
    oi_id = i
    oi_oid = random.randint(1, order_rows)
    oi_pid = random.randint(1, 5000)
    oi_qty = random.randint(1, 10)
    oi_price = round(random.uniform(1, 500), 2)
    oi_disc = round(random.uniform(0, 0.2), 4)
    oi_sub = round(oi_qty * oi_price * (1 - oi_disc), 2)
    batch.append(f"INSERT INTO order_items (oi_id, oi_order_id, oi_product_id, oi_quantity, oi_unit_price, oi_discount, oi_subtotal) VALUES ({oi_id}, {oi_oid}, {oi_pid}, {oi_qty}, {oi_price}, {oi_disc}, {oi_sub})")
    if len(batch) >= batch_size:
        run_sql(';\n'.join(batch))
        batch = []
if batch: run_sql(';\n'.join(batch))
print(f"  ✓ Order items: {item_rows}", flush=True)

print("Data seeding complete.", flush=True)
PYEOF

echo ""
echo "=== Verifying ==="
mysql "${CONN_ARGS[@]}" -e "SELECT COUNT(*) AS customers FROM customers" 2>/dev/null
mysql "${CONN_ARGS[@]}" -e "SELECT COUNT(*) AS orders FROM orders" 2>/dev/null
mysql "${CONN_ARGS[@]}" -e "SELECT COUNT(*) AS order_items FROM order_items" 2>/dev/null

echo ""
echo "DONE"
