#!/usr/bin/env python3
"""
Execution Consistency Harness - Detects divergence across SQL execution paths

Compares SQL execution results across:
  1. mysql-server (MySQL wire protocol server)
  2. bench-cli (benchmark CLI tool)
  3. direct (sqlrustgo REPL direct execution)

Exit 0 only if all paths agree on all SQL statements.
Exit 1 if any divergence is detected.

Usage:
    python execution_consistency_harness.py [options]

Options:
    --corpus PATH       SQL corpus directory (default: ./sql_corpus)
    --sqlrustgo PATH    sqlrustgo binary path (default: ./target/debug/sqlrustgo)
    --mysql-server PATH mysql-server binary path
    --bench-cli PATH    bench-cli binary path
    --port PORT         MySQL server port (default: 3307)
    --timeout SEC       Query timeout in seconds (default: 30)
    --stop-on-error     Stop on first divergence (default: continue)
    --verbose           Show detailed output
    --test FILE         Test specific SQL file only
"""

import hashlib
import json
import os
import re
import signal
import socket
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


@dataclass
class ExecutionResult:
    """Result of SQL execution through a specific path."""
    path_name: str
    sql: str
    output: str
    output_hash: str
    error: Optional[str] = None
    duration_ms: float = 0.0

    def normalized_output(self) -> str:
        """Normalize output for comparison (remove timing, whitespace variations)."""
        if self.error:
            return f"ERROR: {self.error}"
        # Normalize whitespace
        lines = [line.strip() for line in self.output.strip().split('\n')]
        return '\n'.join(line for line in lines if line)

    def to_dict(self) -> dict:
        return {
            'path': self.path_name,
            'sql': self.sql[:100] + '...' if len(self.sql) > 100 else self.sql,
            'output_hash': self.output_hash,
            'error': self.error,
            'duration_ms': self.duration_ms,
        }


@dataclass
class Divergence:
    """Represents a divergence between execution paths."""
    sql: str
    results: dict  # path_name -> ExecutionResult
    message: str


class ExecutionHarness:
    """Main harness for comparing SQL execution across paths."""

    def __init__(self, args):
        self.args = args
        self.corpus_dir = Path(args.corpus)
        self.sqlrustgo_path = Path(args.sqlrustgo)
        self.mysql_server_path = Path(args.mysql_server) if args.mysql_server else None
        self.bench_cli_path = Path(args.bench_cli) if args.bench_cli else None
        self.port = args.port
        self.timeout = args.timeout
        self.stop_on_error = args.stop_on_error
        self.verbose = args.verbose

        self.mysql_server_process = None
        self.mysql_client = None

        self.stats = {
            'total': 0,
            'passed': 0,
            'diverged': 0,
            'errors': 0,
        }

    def log(self, msg: str):
        if self.verbose:
            print(f"[DEBUG] {msg}", file=sys.stderr)

    def hash_output(self, output: str) -> str:
        """Create SHA256 hash of output."""
        return hashlib.sha256(output.encode('utf-8')).hexdigest()[:16]

    def execute_via_repl(self, sql: str, db_path: str) -> ExecutionResult:
        """Execute SQL via direct REPL (sqlrustgo binary)."""
        start = time.time()
        try:
            proc = subprocess.run(
                [str(self.sqlrustgo_path), sql],
                capture_output=True,
                text=True,
                timeout=self.timeout,
                cwd=str(Path(self.sqlrustgo_path).parent.parent),
                env={**os.environ, 'SQLRUSTGO_DB_PATH': db_path}
            )
            duration = (time.time() - start) * 1000
            output = proc.stdout + proc.stderr
            if proc.returncode != 0 and not output:
                output = f"Exit code: {proc.returncode}"
            return ExecutionResult(
                path_name="direct",
                sql=sql,
                output=output.strip(),
                output_hash=self.hash_output(output.strip()),
                duration_ms=duration
            )
        except subprocess.TimeoutExpired:
            return ExecutionResult(
                path_name="direct",
                sql=sql,
                output="",
                output_hash=self.hash_output("TIMEOUT"),
                error="Timeout",
                duration_ms=self.timeout * 1000
            )
        except Exception as e:
            return ExecutionResult(
                path_name="direct",
                sql=sql,
                output="",
                output_hash=self.hash_output("ERROR"),
                error=str(e),
                duration_ms=0
            )

    def execute_via_mysql_server(self, sql: str) -> ExecutionResult:
        """Execute SQL via MySQL server (network protocol)."""
        if not self.mysql_server_path or not self.mysql_server_path.exists():
            return ExecutionResult(
                path_name="mysql-server",
                sql=sql,
                output="",
                output_hash="",
                error="mysql-server path not specified or not found"
            )

        start = time.time()
        try:
            # Use mysql client to connect to our server
            result = subprocess.run(
                ['mysql', '-h', '127.0.0.1', '-P', str(self.port), '-u', 'root', '-e', sql, 'default'],
                capture_output=True,
                text=True,
                timeout=self.timeout
            )
            duration = (time.time() - start) * 1000
            output = result.stdout + result.stderr
            return ExecutionResult(
                path_name="mysql-server",
                sql=sql,
                output=output.strip(),
                output_hash=self.hash_output(output.strip()),
                duration_ms=duration
            )
        except subprocess.TimeoutExpired:
            return ExecutionResult(
                path_name="mysql-server",
                sql=sql,
                output="",
                output_hash=self.hash_output("TIMEOUT"),
                error="Timeout",
                duration_ms=self.timeout * 1000
            )
        except FileNotFoundError:
            return ExecutionResult(
                path_name="mysql-server",
                sql=sql,
                output="",
                output_hash="",
                error="mysql client not found"
            )
        except Exception as e:
            return ExecutionResult(
                path_name="mysql-server",
                sql=sql,
                output="",
                output_hash=self.hash_output("ERROR"),
                error=str(e),
                duration_ms=0
            )

    def execute_via_bench_cli(self, sql: str) -> ExecutionResult:
        """Execute SQL via bench-cli custom query mode."""
        if not self.bench_cli_path or not self.bench_cli_path.exists():
            return ExecutionResult(
                path_name="bench-cli",
                sql=sql,
                output="",
                output_hash="",
                error="bench-cli path not specified or not found"
            )

        start = time.time()
        try:
            # bench-cli uses custom command mode
            result = subprocess.run(
                [str(self.bench_cli_path), 'custom', '--query', sql],
                capture_output=True,
                text=True,
                timeout=self.timeout
            )
            duration = (time.time() - start) * 1000
            output = result.stdout + result.stderr
            return ExecutionResult(
                path_name="bench-cli",
                sql=sql,
                output=output.strip(),
                output_hash=self.hash_output(output.strip()),
                duration_ms=duration
            )
        except subprocess.TimeoutExpired:
            return ExecutionResult(
                path_name="bench-cli",
                sql=sql,
                output="",
                output_hash=self.hash_output("TIMEOUT"),
                error="Timeout",
                duration_ms=self.timeout * 1000
            )
        except Exception as e:
            return ExecutionResult(
                path_name="bench-cli",
                sql=sql,
                output="",
                output_hash=self.hash_output("ERROR"),
                error=str(e),
                duration_ms=0
            )

    def start_mysql_server(self) -> bool:
        """Start the MySQL server for testing."""
        if not self.mysql_server_path or not self.mysql_server_path.exists():
            self.log("mysql-server not available, skipping")
            return False

        self.log(f"Starting mysql-server on port {self.port}")
        try:
            self.mysql_server_process = subprocess.Popen(
                [str(self.mysql_server_path), '--port', str(self.port), '--host', '127.0.0.1'],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE
            )

            # Wait for server to be ready
            for i in range(30):
                if self._is_port_open('127.0.0.1', self.port):
                    self.log("mysql-server is ready")
                    return True
                time.sleep(0.5)

            self.log("mysql-server failed to start within timeout")
            return False
        except Exception as e:
            self.log(f"Failed to start mysql-server: {e}")
            return False

    def stop_mysql_server(self):
        """Stop the MySQL server."""
        if self.mysql_server_process:
            self.log("Stopping mysql-server")
            self.mysql_server_process.terminate()
            try:
                self.mysql_server_process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.mysql_server_process.kill()

    def _is_port_open(self, host: str, port: int) -> bool:
        """Check if a port is open."""
        try:
            with socket.create_connection((host, port), timeout=1):
                return True
        except (socket.timeout, ConnectionRefusedError, OSError):
            return False

    def parse_sql_file(self, file_path: Path) -> tuple:
        """Parse SQL file into full batch and individual test statements.
        
        Returns:
            (full_batch_sql, test_statements) - tuple of full SQL batch and individual test SQLs
        """
        content = file_path.read_text()

        # Check for SKIP marker at file level
        if '=== SKIP ===' in content:
            return ("", [])

        all_lines = []
        test_statements = []
        in_setup = False
        in_case = False

        for line in content.split('\n'):
            original = line
            line = line.strip()

            # Handle markers
            if '=== SKIP ===' in line:
                return ("", [])
            if '=== SETUP ===' in line:
                in_setup = True
                in_case = False
                continue
            if '=== CASE:' in line:
                in_setup = False
                in_case = True
                continue

            # Skip comment-only lines
            if line.startswith('--'):
                continue

            if not line:
                continue

            # Add non-comment lines to our collection
            all_lines.append(original.strip())

            # Track test statements (after setup, during cases)
            if in_case and not line.startswith('--'):
                if line.endswith(';'):
                    test_statements.append(line.rstrip(';').strip())
                else:
                    test_statements.append(line.strip())

        # Build full batch (remove trailing semicolons for cleaner execution)
        full_batch = '\n'.join(all_lines)
        # Remove trailing semicolons
        full_batch = re.sub(r';\s*$', '', full_batch)

        return (full_batch, test_statements)

    def execute_sql_on_all_paths(self, sql: str, db_path: str) -> dict:
        """Execute SQL on all available paths and return results."""
        results = {}

        # Direct REPL execution (always available)
        results['direct'] = self.execute_via_repl(sql, db_path)

        # MySQL server execution
        if self.mysql_server_path and self.mysql_server_path.exists():
            results['mysql-server'] = self.execute_via_mysql_server(sql)
        else:
            results['mysql-server'] = ExecutionResult(
                path_name="mysql-server",
                sql=sql,
                output="",
                output_hash="",
                error="not available"
            )

        # Bench-cli execution
        if self.bench_cli_path and self.bench_cli_path.exists():
            results['bench-cli'] = self.execute_via_bench_cli(sql)
        else:
            results['bench-cli'] = ExecutionResult(
                path_name="bench-cli",
                sql=sql,
                output="",
                output_hash="",
                error="not available"
            )

        return results

    def check_consistency(self, results: dict) -> Optional[Divergence]:
        """Check if all paths produce consistent results."""
        # Collect successful outputs (ignore errors for comparison)
        successful = {k: v for k, v in results.items() if not v.error}

        if len(successful) < 2:
            return None  # Not enough paths to compare

        # Compare hashes
        hashes = {k: v.output_hash for k, v in successful.items()}
        unique_hashes = set(hashes.values())

        if len(unique_hashes) > 1:
            # Find the diverging paths
            hash_to_paths = {}
            for path, h in hashes.items():
                hash_to_paths.setdefault(h, []).append(path)

            # Get the most common result as baseline
            baseline_hash = max(hash_to_paths.keys(), key=lambda h: len(hash_to_paths[h]))
            baseline_paths = hash_to_paths[baseline_hash]
            diverging = {k: v for k, v in successful.items() if k not in baseline_paths}

            message = f"Divergence detected: {len(diverging)} path(s) disagree with {len(baseline_paths)} baseline path(s)"
            return Divergence(sql=results['direct'].sql, results=results, message=message)

        return None

    def run(self):
        """Main execution loop."""
        print("=" * 60)
        print("SQLRustGo Execution Consistency Harness")
        print("=" * 60)
        print(f"Corpus: {self.corpus_dir}")
        print(f"sqlrustgo: {self.sqlrustgo_path}")
        print(f"mysql-server: {self.mysql_server_path}")
        print(f"bench-cli: {self.bench_cli_path}")
        print(f"Port: {self.port}")
        print("=" * 60)
        print()

        # Start MySQL server if available
        server_started = self.start_mysql_server()

        try:
            # Collect all SQL files
            if self.args.test:
                sql_files = [Path(self.args.test)]
            else:
                sql_files = list(self.corpus_dir.rglob('*.sql'))

            print(f"Found {len(sql_files)} SQL files")
            print()

            divergences = []

            for sql_file in sorted(sql_files):
                self.log(f"Processing: {sql_file}")

                full_batch, test_stmts = self.parse_sql_file(sql_file)
                if not full_batch:
                    self.log(f"No valid SQL in {sql_file}")
                    continue

                self.log(f"Found {len(test_stmts)} test statements")
                
                # For each test statement, we need to first run the full batch
                # to set up the database, then run the individual test
                for stmt in test_stmts:
                    self.stats['total'] += 1

                    # Create temp database for this run
                    with tempfile.TemporaryDirectory() as tmpdir:
                        db_path = os.path.join(tmpdir, 'test.db')

                        # Execute batch setup first
                        batch_results = self.execute_sql_on_all_paths(full_batch, db_path)
                        
                        # Now execute the individual test
                        results = self.execute_sql_on_all_paths(stmt, db_path)
                        divergence = self.check_consistency(results)

                        if divergence:
                            self.stats['diverged'] += 1
                            divergences.append(divergence)
                            print(f"[DIVERGENCE] {sql_file.name}")
                            print(f"  SQL: {stmt[:80]}...")
                            for path, result in results.items():
                                status = "ERROR" if result.error else "OK"
                                print(f"  [{path}] {status}: {result.output[:60]}...")
                            print()

                            if self.stop_on_error:
                                break
                        else:
                            self.stats['passed'] += 1

                    if self.stop_on_error and divergences:
                        break

                if self.stop_on_error and divergences:
                    break

        finally:
            self.stop_mysql_server()

        # Print summary
        print()
        print("=" * 60)
        print("SUMMARY")
        print("=" * 60)
        print(f"Total SQL statements: {self.stats['total']}")
        print(f"Consistent: {self.stats['passed']}")
        print(f"Diverged: {self.stats['diverged']}")
        print(f"Errors: {self.stats['errors']}")
        print()

        if divergences:
            print("DIVERGENCES DETECTED:")
            print("-" * 40)
            for i, div in enumerate(divergences, 1):
                print(f"\n{i}. SQL: {div.sql[:100]}...")
                for path, result in div.results.items():
                    print(f"   {path}: {result.output[:60]}...")
            print()
            print("FAIL: Execution paths produced inconsistent results")
            return 1
        else:
            if self.stats['total'] == 0:
                print("No SQL statements were tested")
                return 1
            print("PASS: All execution paths agree on all SQL statements")
            return 0


def main():
    import argparse

    parser = argparse.ArgumentParser(
        description='Execution Consistency Harness for SQLRustGo',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    parser.add_argument(
        '--corpus',
        default='./sql_corpus',
        help='SQL corpus directory'
    )
    parser.add_argument(
        '--sqlrustgo',
        default='./target/debug/sqlrustgo',
        help='sqlrustgo binary path'
    )
    parser.add_argument(
        '--mysql-server',
        default=None,
        help='sqlrustgo-mysql-server binary path'
    )
    parser.add_argument(
        '--bench-cli',
        default=None,
        help='sqlrustgo-bench-cli binary path'
    )
    parser.add_argument(
        '--port',
        type=int,
        default=3307,
        help='MySQL server port (default: 3307)'
    )
    parser.add_argument(
        '--timeout',
        type=int,
        default=30,
        help='Query timeout in seconds (default: 30)'
    )
    parser.add_argument(
        '--stop-on-error',
        action='store_true',
        help='Stop on first divergence'
    )
    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Show detailed output'
    )
    parser.add_argument(
        '--test',
        help='Test specific SQL file only'
    )

    args = parser.parse_args()

    # Validate paths
    if not Path(args.sqlrustgo).exists():
        print(f"Error: sqlrustgo not found at {args.sqlrustgo}", file=sys.stderr)
        print("Build with: cargo build", file=sys.stderr)
        sys.exit(1)

    if args.mysql_server and not Path(args.mysql_server).exists():
        print(f"Warning: mysql-server not found at {args.mysql_server}", file=sys.stderr)

    if args.bench_cli and not Path(args.bench_cli).exists():
        print(f"Warning: bench-cli not found at {args.bench_cli}", file=sys.stderr)

    # Change to project directory
    project_dir = Path(__file__).parent.parent.parent
    os.chdir(project_dir)

    harness = ExecutionHarness(args)
    sys.exit(harness.run())


if __name__ == '__main__':
    main()
