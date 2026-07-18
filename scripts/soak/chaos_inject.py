#!/usr/bin/env python3
"""
chaos_inject.py — Chaos Engineering Injection Controller

Provides fault injection capabilities for SOAK testing:
- I/O latency injection (Linux tc qdisc)
- Memory pressure (stress-ng)
- Process kill (kill -9)

Usage:
    python3 chaos_inject.py --type io_latency --duration 60
    python3 chaos_inject.py --type memory_pressure --workers 4 --megs 512
    python3 chaos_inject.py --type kill9 --pid 12345
    python3 chaos_inject.py --cleanup  # Clean up all injected faults
"""

import argparse
import json
import os
import platform
import random
import signal
import socket
import subprocess
import sys
import time
from dataclasses import dataclass, asdict
from typing import List, Optional


@dataclass
class ChaosResult:
    experiment: str
    success: bool
    recovery_time_ms: Optional[int] = None
    error: Optional[str] = None
    platform: str = ""


class ChaosController:
    """Unified chaos injection controller."""

    def __init__(self, sudo: bool = False):
        self.sudo = sudo
        self.platform = platform.system()
        self.active_experiments: List[str] = []
        self.server_pid: Optional[int] = None

    def _run(self, cmd: List[str], check: bool = True, timeout: int = 30) -> subprocess.CompletedProcess:
        """Run a command, optionally with sudo."""
        full_cmd = cmd
        if self.sudo and os.geteuid() != 0:
            full_cmd = ["sudo"] + cmd
        return subprocess.run(full_cmd, check=check, capture_output=True, text=True, timeout=timeout)

    def _is_linux(self) -> bool:
        return self.platform == "Linux"

    def inject_io_latency(self, delay_ms: int = 100, device: str = "eth0") -> ChaosResult:
        """
        Inject I/O latency using Linux tc qdisc.
        Simulates disk delay by adding network delay on loopback.
        """
        if not self._is_linux():
            return ChaosResult(
                experiment="io_latency",
                success=False,
                error="I/O latency injection only supported on Linux"
            )

        try:
            # Check if tc is available
            self._run(["tc", "-V"], check=False)

            # Add 100ms delay on loopback (safe to test with)
            self._run(
                ["tc", "qdisc", "add", "dev", "lo", "root", "netem", "delay", f"{delay_ms}ms"],
                check=False
            )

            self.active_experiments.append("io_latency")
            return ChaosResult(
                experiment="io_latency",
                success=True,
                platform=self.platform
            )

        except subprocess.TimeoutExpired:
            return ChaosResult(experiment="io_latency", success=False, error="Command timeout")
        except Exception as e:
            return ChaosResult(experiment="io_latency", success=False, error=str(e))

    def inject_memory_pressure(self, workers: int = 4, megs: int = 512, duration: int = 60) -> ChaosResult:
        """
        Inject memory pressure using stress-ng.
        """
        if not self._is_linux():
            return ChaosResult(
                experiment="memory_pressure",
                success=False,
                error="Memory pressure injection only supported on Linux"
            )

        try:
            # Check if stress-ng is available
            stress_check = self._run(["which", "stress-ng"], check=False)
            if stress_check.returncode != 0:
                return ChaosResult(
                    experiment="memory_pressure",
                    success=False,
                    error="stress-ng not installed. Install with: sudo apt install stress-ng"
                )

            # Run stress-ng in background
            proc = subprocess.Popen(
                ["stress-ng", "--vm", str(workers), "--vm-bytes", f"{megs}M", "-t", f"{duration}s"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE
            )

            self.active_experiments.append("memory_pressure")
            return ChaosResult(
                experiment="memory_pressure",
                success=True,
                platform=self.platform
            )

        except FileNotFoundError:
            return ChaosResult(experiment="memory_pressure", success=False, error="stress-ng not found")
        except Exception as e:
            return ChaosResult(experiment="memory_pressure", success=False, error=str(e))

    def kill_server_process(self, pid: Optional[int] = None) -> ChaosResult:
        """
        Kill the server process (or provided PID) with SIGKILL.
        """
        target_pid = pid or self.server_pid
        if not target_pid:
            return ChaosResult(
                experiment="kill9",
                success=False,
                error="No PID provided and no server PID set"
            )

        try:
            # Verify process exists
            os.kill(target_pid, 0)  # Signal 0 just checks existence

            # Send SIGKILL
            os.kill(target_pid, signal.SIGKILL)

            self.active_experiments.append("kill9")
            return ChaosResult(
                experiment="kill9",
                success=True,
                platform=self.platform
            )

        except ProcessLookupError:
            return ChaosResult(experiment="kill9", success=False, error=f"Process {target_pid} not found")
        except PermissionError:
            return ChaosResult(experiment="kill9", success=False, error=f"Permission denied to kill {target_pid}")
        except Exception as e:
            return ChaosResult(experiment="kill9", success=False, error=str(e))

    def verify_recovery(self, host: str = "127.0.0.1", port: int = 3306, timeout: int = 5) -> ChaosResult:
        """
        Verify that the server has recovered within the timeout period.
        """
        start = time.time()
        while time.time() - start < timeout:
            try:
                sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                sock.settimeout(1)
                result = sock.connect_ex((host, port))
                sock.close()
                if result == 0:
                    recovery_time_ms = int((time.time() - start) * 1000)
                    return ChaosResult(
                        experiment="recovery",
                        success=True,
                        recovery_time_ms=recovery_time_ms,
                        platform=self.platform
                    )
            except Exception:
                pass
            time.sleep(0.5)

        return ChaosResult(
            experiment="recovery",
            success=False,
            error=f"Server did not recover within {timeout}s",
            platform=self.platform
        )

    def cleanup(self) -> ChaosResult:
        """
        Clean up all injected faults.
        """
        if not self._is_linux():
            return ChaosResult(experiment="cleanup", success=True, platform=self.platform)

        results = []
        try:
            # Remove tc qdisc delay
            result = self._run(
                ["tc", "qdisc", "del", "dev", "lo", "root"],
                check=False
            )
            if result.returncode == 0:
                results.append("tc qdisc cleaned")

            # Kill any stress-ng processes
            result = self._run(
                ["pkill", "-9", "stress-ng"],
                check=False
            )
            if result.returncode == 0:
                results.append("stress-ng processes killed")

            self.active_experiments.clear()
            return ChaosResult(
                experiment="cleanup",
                success=True,
                platform=self.platform
            )

        except Exception as e:
            return ChaosResult(experiment="cleanup", success=False, error=str(e))


def main():
    parser = argparse.ArgumentParser(description="Chaos Injection Controller for SOAK Testing")
    subparsers = parser.add_subparsers(dest="command", help="Commands")

    # Inject subcommand
    inject_parser = subparsers.add_parser("inject", help="Inject a chaos experiment")
    inject_parser.add_argument("--type", required=True,
                               choices=["io_latency", "memory_pressure", "kill9"],
                               help="Type of chaos experiment")
    inject_parser.add_argument("--delay-ms", type=int, default=100,
                               help="I/O latency delay in ms (default: 100)")
    inject_parser.add_argument("--workers", type=int, default=4,
                               help="Number of stress-ng workers (default: 4)")
    inject_parser.add_argument("--megs", type=int, default=512,
                               help="Memory pressure in MB (default: 512)")
    inject_parser.add_argument("--pid", type=int, help="PID to kill (for kill9)")
    inject_parser.add_argument("--duration", type=int, default=60,
                               help="Duration in seconds (for memory pressure)")
    inject_parser.add_argument("--sudo", action="store_true",
                               help="Use sudo for privileged operations")

    # Recovery subcommand
    recover_parser = subparsers.add_parser("verify-recovery", help="Verify server recovery")
    recover_parser.add_argument("--host", default="127.0.0.1", help="Server host")
    recover_parser.add_argument("--port", type=int, default=3306, help="Server port")
    recover_parser.add_argument("--timeout", type=int, default=5, help="Timeout in seconds")
    recover_parser.add_argument("--sudo", action="store_true", help="Use sudo")

    # Cleanup subcommand
    cleanup_parser = subparsers.add_parser("cleanup", help="Clean up all injected faults")
    cleanup_parser.add_argument("--sudo", action="store_true", help="Use sudo")

    args = parser.parse_args()

    controller = ChaosController(sudo=getattr(args, "sudo", False))

    if args.command == "inject":
        if args.type == "io_latency":
            result = controller.inject_io_latency(delay_ms=args.delay_ms)
        elif args.type == "memory_pressure":
            result = controller.inject_memory_pressure(workers=args.workers, megs=args.megs, duration=args.duration)
        elif args.type == "kill9":
            result = controller.kill_server_process(pid=args.pid)
        else:
            print(f"Unknown experiment type: {args.type}")
            sys.exit(1)
        print(json.dumps(asdict(result), indent=2))

    elif args.command == "verify-recovery":
        result = controller.verify_recovery(host=args.host, port=args.port, timeout=args.timeout)
        print(json.dumps(asdict(result), indent=2))

    elif args.command == "cleanup":
        result = controller.cleanup()
        print(json.dumps(asdict(result), indent=2))

    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
