use clap::Parser;
use rand::Rng;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value_t = 1)]
    scale: u32,

    #[arg(long, default_value = ".")]
    output: String,
}

struct TpchDataGenerator {
    scale: u32,
    output_dir: PathBuf,
}

impl TpchDataGenerator {
    fn new(scale: u32, output: String) -> Self {
        Self {
            scale,
            output_dir: PathBuf::from(output),
        }
    }

    fn generate_all(&self) -> std::io::Result<()> {
        println!("Generating TPC-H data for SF={}", self.scale);

        std::fs::create_dir_all(&self.output_dir)?;

        let row_counts = self.get_row_counts();

        self.generate_region()?;
        self.generate_nation()?;
        self.generate_supplier(100 * row_counts.customer / 1500)?;
        self.generate_customer(row_counts.customer)?;
        self.generate_part(2000 * row_counts.customer / 1500)?;
        self.generate_partsupp(8000 * row_counts.customer / 1500)?;
        self.generate_orders(row_counts.orders)?;
        self.generate_lineitem(row_counts.lineitem)?;

        println!("Data generation complete!");
        println!("Files created in: {}", self.output_dir.display());

        Ok(())
    }

    fn get_row_counts(&self) -> RowCounts {
        let sf = self.scale as f64;
        RowCounts {
            customer: (1500.0 * sf) as usize,
            orders: (15000.0 * sf) as usize,
            lineitem: (60000.0 * sf) as usize,
        }
    }

    // TPC-H spec: region has 5 rows (1 row per region, fixed).
    fn generate_region(&self) -> std::io::Result<()> {
        let mut tbl = File::create(self.output_dir.join("region.tbl"))?;
        let names = ["AFRICA", "AMERICA", "ASIA", "EUROPE", "MIDDLE EAST"];
        for (i, name) in names.iter().enumerate() {
            writeln!(tbl, "{}|{}|lar deposits. Special", i, name)?;
        }
        println!("  region.tbl: 5 rows");
        Ok(())
    }

    // TPC-H spec: nation has 25 rows (5 per region).
    // Order matters: regionkey = i/5. The spec defines the canonical list
    // per region, NOT alphabetical, so use the exact TPC-H names.
    fn generate_nation(&self) -> std::io::Result<()> {
        let mut tbl = File::create(self.output_dir.join("nation.tbl"))?;
        let names = [
            // AFRICA (regionkey=0)
            "ALGERIA", "ETHIOPIA", "KENYA", "MOROCCO", "MOZAMBIQUE",
            // AMERICA (regionkey=1)
            "ARGENTINA", "BRAZIL", "CANADA", "PERU", "UNITED STATES",
            // ASIA (regionkey=2)
            "CHINA", "INDIA", "INDONESIA", "JAPAN", "VIETNAM",
            // EUROPE (regionkey=3)
            "FRANCE", "GERMANY", "ROMANIA", "RUSSIA", "UNITED KINGDOM",
            // MIDDLE EAST (regionkey=4)
            "EGYPT", "IRAN", "IRAQ", "JORDAN", "SAUDI ARABIA",
        ];
        for (i, name) in names.iter().enumerate() {
            let regionkey = i / 5;
            writeln!(tbl, "{}|{}|{}| haggle. carefully final", i, name, regionkey)?;
        }
        println!("  nation.tbl: 25 rows");
        Ok(())
    }

    // TPC-H spec: supplier is SF * 10000 (SF=1 → 100 rows; 4-way uses 100)
    fn generate_supplier(&self, count: usize) -> std::io::Result<()> {
        let mut tbl = File::create(self.output_dir.join("supplier.tbl"))?;
        let mut rng = rand::thread_rng();
        for i in 1..=count {
            let suppkey = i;
            let name = format!("Supplier#{:09}", suppkey);
            let address = format!(
                "{} {} {} {} {}",
                rng.gen::<u32>() % 100,
                self.random_string(10, &mut rng),
                self.random_string(4, &mut rng),
                self.random_number(9, &mut rng),
                self.random_number(6, &mut rng)
            );
            let nationkey = (rng.gen::<u32>() % 25) + 1; // 1..=25
            let phone = format!(
                "{}-{}-{}",
                self.random_number(3, &mut rng),
                self.random_number(4, &mut rng),
                self.random_number(4, &mut rng)
            );
            let acctbal = (rng.gen::<f64>() * 9999.99 - 999.99).round() / 100.0;
            let comment = self.random_string(63, &mut rng);
            writeln!(
                tbl,
                "{}|{}|{}|{}|{}|{:.2}|{}",
                suppkey, name, address, nationkey, phone, acctbal, comment
            )?;
        }
        println!("  supplier.tbl: {} rows", count);
        Ok(())
    }

    // TPC-H spec: part is SF * 200000 (SF=1 → 2000 rows)
    fn generate_part(&self, count: usize) -> std::io::Result<()> {
        let mut tbl = File::create(self.output_dir.join("part.tbl"))?;
        let mut rng = rand::thread_rng();
        let mfgrs = ["Manufacturer#1", "Manufacturer#2", "Manufacturer#3", "Manufacturer#4", "Manufacturer#5"];
        let brands = [
            "Brand#11", "Brand#12", "Brand#13", "Brand#14", "Brand#15",
            "Brand#21", "Brand#22", "Brand#23", "Brand#24", "Brand#25",
            "Brand#31", "Brand#32", "Brand#33", "Brand#34", "Brand#35",
            "Brand#41", "Brand#42", "Brand#43", "Brand#44", "Brand#45",
            "Brand#51", "Brand#52", "Brand#53", "Brand#54", "Brand#55",
        ];
        let containers = [
            "SM CASE", "LG BOX", "MED BAG", "MED BOX", "LG CASE",
            "SM PACK", "SM PKG", "MED PACK", "WRAP PKG", "SM JAR",
        ];
        let types = [
            "STANDARD POLISHED TIN", "SMALL POLISHED COPPER", "MEDIUM PLATED STEEL",
            "STANDARD BRUSHED COPPER", "SMALL ANODIZED NICKEL", "MEDIUM ANODIZED TIN",
            "LARGE POLISHED BRASS", "SMALL PLATED COPPER", "MEDIUM BRUSHED TIN",
            "LARGE ANODIZED STEEL", "ECONOMY BRUSHED NICKEL", "PROMO ANODIZED BRASS",
        ];
        for i in 1..=count {
            let partkey = i;
            let name = format!(
                "{} {} {}",
                types[rng.gen::<usize>() % types.len()],
                self.random_string(7, &mut rng).to_lowercase(),
                self.random_string(7, &mut rng).to_lowercase()
            );
            let mfgr = mfgrs[rng.gen::<usize>() % mfgrs.len()].to_string();
            let brand = brands[rng.gen::<usize>() % brands.len()].to_string();
            let ptype = types[rng.gen::<usize>() % types.len()];
            let size = 1 + rng.gen::<u32>() % 50;
            let container = containers[rng.gen::<usize>() % containers.len()].to_string();
            let retailprice = (rng.gen::<f64>() * 2099.0 + 900.0).round() / 100.0;
            let comment = self.random_string(7, &mut rng);
            writeln!(
                tbl,
                "{}|{}|{}|{}|{}|{}|{}|{:.2}|{}",
                partkey, name, mfgr, brand, ptype, size, container, retailprice, comment
            )?;
        }
        println!("  part.tbl: {} rows", count);
        Ok(())
    }

    // TPC-H spec: partsupp is SF * 800000 (SF=1 → 8000 rows)
    fn generate_partsupp(&self, count: usize) -> std::io::Result<()> {
        let mut tbl = File::create(self.output_dir.join("partsupp.tbl"))?;
        let mut rng = rand::thread_rng();
        // 4-way harness: count=8000 for SF=1. distribute over (partkey, suppkey) pairs.
        // partsupp key is (ps_partkey, ps_suppkey). partkey 1..=2000, suppkey 1..=100
        // → 200000 pairs. We sample 8000 of them.
        let part_count = 2000usize;
        let supp_count = 100usize;
        for i in 1..=count {
            let ps_partkey = (i % part_count) + 1;
            let ps_suppkey = ((i / part_count) % supp_count) + 1;
            let ps_availqty = rng.gen::<u32>() % 9999 + 1;
            let ps_supplycost = (rng.gen::<f64>() * 1000.0 + 1.0).round() / 100.0;
            let ps_comment = self.random_string(63, &mut rng);
            writeln!(
                tbl,
                "{}|{}|{}|{:.2}|{}",
                ps_partkey, ps_suppkey, ps_availqty, ps_supplycost, ps_comment
            )?;
        }
        println!("  partsupp.tbl: {} rows", count);
        Ok(())
    }

    fn generate_customer(&self, count: usize) -> std::io::Result<()> {
        let mut file = File::create(self.output_dir.join("customer.csv"))?;
        let mut tbl = File::create(self.output_dir.join("customer.tbl"))?;
        writeln!(
            file,
            "c_custkey,c_name,c_address,c_nationkey,c_phone,c_acctbal,c_mktsegment,c_comment"
        )?;

        let mut rng = rand::thread_rng();
        let segments = [
            "AUTOMOBILE",
            "BUILDING",
            "FURNITURE",
            "MACHINERY",
            "HOUSEHOLD",
        ];

        for i in 1..=count {
            let custkey = i;
            let name = format!("Customer#{:09}", custkey);
            let address = format!(
                "{} {} {} {} {}",
                rng.gen::<u32>() % 100,
                self.random_string(10, &mut rng),
                self.random_string(4, &mut rng),
                self.random_number(9, &mut rng),
                self.random_number(6, &mut rng)
            );
            let nationkey = rng.gen::<u32>() % 25;
            let phone = format!(
                "{}-{}-{}",
                self.random_number(3, &mut rng),
                self.random_number(4, &mut rng),
                self.random_number(4, &mut rng)
            );
            let acctbal = (rng.gen::<f64>() * 9999.99 - 999.99).round() / 100.0;
            let mktsegment = segments[rng.gen::<usize>() % 5];
            let comment = self.random_string(29, &mut rng);

            writeln!(
                file,
                "{},{},{},{},{},{:.2},{},{}",
                custkey, name, address, nationkey, phone, acctbal, mktsegment, comment
            )?;
            // TBL: no header, pipe-separated
            writeln!(
                tbl,
                "{}|{}|{}|{}|{}|{:.2}|{}|{}",
                custkey, name, address, nationkey, phone, acctbal, mktsegment, comment
            )?;
        }

        println!("  customer.csv: {} rows", count);
        println!("  customer.tbl: {} rows", count);
        Ok(())
    }

    fn generate_orders(&self, count: usize) -> std::io::Result<()> {
        let mut file = File::create(self.output_dir.join("orders.csv"))?;
        let mut tbl = File::create(self.output_dir.join("orders.tbl"))?;
        writeln!(file, "o_orderkey,o_custkey,o_orderstatus,o_totalprice,o_orderdate,o_orderpriority,o_clerk,o_shippriority,o_comment")?;

        let mut rng = rand::thread_rng();
        let statuses = ["O", "F", "P"];
        let priorities = ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"];

        for i in 1..=count {
            let orderkey = i;
            let custkey = (rng.gen::<u32>() % 1500) + 1;
            let status = statuses[rng.gen::<usize>() % 3];
            let totalprice = (rng.gen::<f64>() * 99999.99).round() / 100.0;
            let orderdate = self.random_date(1992, 1998, &mut rng);
            let priority = priorities[rng.gen::<usize>() % 5];
            let clerk = format!("Clerk#{:06}", rng.gen::<u32>() % 1000);
            let shippriority = rng.gen::<u32>() % 100;
            let comment = self.random_string(19, &mut rng);

            writeln!(
                file,
                "{},{},{},{:.2},{},{},{},{},{}",
                orderkey,
                custkey,
                status,
                totalprice,
                orderdate,
                priority,
                clerk,
                shippriority,
                comment
            )?;
            // TBL: no header, pipe-separated
            writeln!(
                tbl,
                "{}|{}|{}|{:.2}|{}|{}|{}|{}|{}",
                orderkey,
                custkey,
                status,
                totalprice,
                orderdate,
                priority,
                clerk,
                shippriority,
                comment
            )?;
        }

        println!("  orders.csv: {} rows", count);
        println!("  orders.tbl: {} rows", count);
        Ok(())
    }

    fn generate_lineitem(&self, count: usize) -> std::io::Result<()> {
        // TPC-H spec: 16 columns. l_linestatus MUST be column 10 (between
        // l_returnflag and l_shipdate) so that GROUP BY l_returnflag,
        // l_linestatus (Q1) sees the right value.
        let mut csv = File::create(self.output_dir.join("lineitem.csv"))?;
        // Also write TBL (pipe-delimited, no header) for the 4-way harness
        let mut tbl = File::create(self.output_dir.join("lineitem.tbl"))?;

        let mut rng = rand::thread_rng();
        let return_flags = ["N", "R", "A"];
        let line_statuses = ["O", "F"]; // O=open, F=filled (TPC-H spec)
        let shipmodes = ["AIR", "AIR REG", "FOB", "MAIL", "RAIL", "SHIP", "TRUCK"];
        let instructs = [
            "DELIVER IN PERSON",
            "NONE",
            "TAKE BACK RETURN",
            "COLLECT COD",
        ];

        writeln!(csv, "l_orderkey,l_partkey,l_suppkey,l_linenumber,l_quantity,l_extendedprice,l_discount,l_tax,l_returnflag,l_linestatus,l_shipdate,l_commitdate,l_receiptdate,l_shipinstruct,l_shipmode,l_comment")?;

        for i in 1..=count {
            let orderkey = ((i as f64 / 15.0).floor() as usize) + 1;
            let partkey = ((i as f64 / 4.0).floor() as usize) % 2000 + 1;
            let suppkey = ((i as f64 / 8.0).floor() as usize) % 1000 + 1;
            let linenumber = ((i - 1) % 7) + 1;
            let quantity = (rng.gen::<u32>() % 50) + 1;
            let extendedprice =
                (quantity as f64 * (rng.gen::<f64>() * 1000.0 + 100.0)).round() / 100.0;
            // TPC-H spec: discount 0.00..0.10, tax 0.00..0.08. Use a
            // 0-100 integer (0-10000) for fine-grained control and
            // divide to get proper distribution. Old code used
            // (rng * 0.10).round() / 100.0 which always rounded to 0
            // — this is the root cause of TPC-H Q6/Q19 row_count
            // mismatch in the 4-way harness (sibling Sprint 1.5).
            let discount = (rng.gen::<u32>() % 11) as f64 / 100.0; // 0.00..0.10
            let tax = (rng.gen::<u32>() % 9) as f64 / 100.0; // 0.00..0.08
            let returnflag = return_flags[rng.gen::<usize>() % 3];
            let linestatus = line_statuses[rng.gen::<usize>() % 2];
            let shipdate = self.random_date(1992, 1998, &mut rng);
            let commitdate = self.random_date(1992, 1998, &mut rng);
            let receiptdate = self.random_date(1993, 1999, &mut rng);
            let instruct = instructs[rng.gen::<usize>() % 4];
            let shipmode = shipmodes[rng.gen::<usize>() % 7];
            let comment = self.random_string(14, &mut rng);

            writeln!(
                csv,
                "{},{},{},{},{},{:.2},{:.2},{:.2},{},{},{},{},{},{},{},{}",
                orderkey,
                partkey,
                suppkey,
                linenumber,
                quantity,
                extendedprice,
                discount,
                tax,
                returnflag,
                linestatus,
                shipdate,
                commitdate,
                receiptdate,
                instruct,
                shipmode,
                comment
            )?;
            // TBL: pipe-delimited, NO header, trailing newline only.
            // 16 columns in spec order.
            writeln!(
                tbl,
                "{}|{}|{}|{}|{}|{:.2}|{:.2}|{:.2}|{}|{}|{}|{}|{}|{}|{}|{}",
                orderkey,
                partkey,
                suppkey,
                linenumber,
                quantity,
                extendedprice,
                discount,
                tax,
                returnflag,
                linestatus,
                shipdate,
                commitdate,
                receiptdate,
                instruct,
                shipmode,
                comment
            )?;
        }

        println!("  lineitem.csv: {} rows", count);
        println!("  lineitem.tbl: {} rows", count);
        Ok(())
    }

    fn random_string(&self, len: usize, rng: &mut impl Rng) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        (0..len)
            .map(|_| {
                let idx = (rng.gen::<u32>() % CHARSET.len() as u32) as usize;
                CHARSET[idx] as char
            })
            .collect()
    }

    fn random_number(&self, len: usize, rng: &mut impl Rng) -> String {
        (0..len)
            .map(|_| char::from(b'0' + (rng.gen::<u32>() % 10) as u8))
            .collect()
    }

    fn random_date(&self, year_start: u32, year_end: u32, rng: &mut impl Rng) -> String {
        let year = year_start + rng.gen::<u32>() % (year_end - year_start + 1);
        let month = 1 + rng.gen::<u32>() % 12;
        let day = 1 + rng.gen::<u32>() % 28;
        format!("{:04}-{:02}-{:02}", year, month, day)
    }
}

struct RowCounts {
    customer: usize,
    orders: usize,
    lineitem: usize,
}

fn main() {
    let args = Args::parse();

    println!("TPC-H Data Generator");
    println!("====================");

    let generator = TpchDataGenerator::new(args.scale, args.output);

    if let Err(e) = generator.generate_all() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
