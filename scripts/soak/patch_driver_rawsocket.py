#!/usr/bin/env python3
"""Patch tpch_mixed_soak_driver.py to use raw socket instead of mysql CLI."""
import re

with open('scripts/soak/tpch_mixed_soak_driver.py', 'r') as f:
    content = f.read()

if 'class RawSocket' in content:
    print("Already patched")
    exit(0)

rawsocket_code = '''
import socket, struct

class RawSocket:
    """Minimal MySQL raw-socket client for SQLRustGo binary mode."""
    def __init__(self, host, port, user):
        self.host = host; self.port = port; self.user = user
        self._sock = None; self._seq = 0

    def _recv_packet(self):
        header = b''
        while len(header) < 4:
            d = self._sock.recv(4 - len(header))
            if not d: return None, b''
            header += d
        length = header[0] | (header[1] << 8) | (header[2] << 16)
        seq = header[3]; body = b''
        while len(body) < length:
            d = self._sock.recv(length - len(body))
            if not d: return seq, body
            body += d
        return seq, body

    def _send_packet(self, payload, seq=None):
        if seq is None: seq = self._seq
        pkt = struct.pack('<I', len(payload))[:3] + bytes([seq])
        self._sock.sendall(pkt + payload)
        self._seq = (seq + 1) % 256

    def connect(self):
        self._sock = socket.socket()
        self._sock.settimeout(30)
        self._sock.connect((self.host, self.port))
        seq, body = self._recv_packet()  # server handshake
        auth = struct.pack('<B', 0x01) + struct.pack('<I', 0) * 2 + b'\\x00' * 24 + b'\\x00'
        self._send_packet(auth, seq + 1)
        seq, body = self._recv_packet()
        if body and body[0] != 0x00:
            raise Exception(f"Auth failed: {body.hex()}")
        return self

    def execute(self, sql):
        """Execute SQL, return (ok, output_text)."""
        try:
            self._send_packet(bytes([0x03]) + sql.encode())
            output = []
            while True:
                seq, body = self._recv_packet()
                if not body:
                    return False, "no response"
                if body[0] == 0xff:
                    errno = body[1] | body[2] << 8
                    msg = body[5:].decode('utf8', errors='replace')
                    return False, f"ERROR {errno}: {msg}"
                if body[0] == 0x00:
                    return True, "\\n".join(output)
                if body[0] == 0xfe:
                    break
                # column def or row - skip
            return True, "\\n".join(output)
        except socket.timeout:
            return False, "timeout"
        except Exception as e:
            return False, str(e)

    def close(self):
        if self._sock:
            try: self._sock.close()
            except: pass

    def __enter__(self): return self.connect()
    def __exit__(self, *args): self.close()


_thread_conns = {}

def run_mysql(host: str, port: int, user: str, sql: str,
              db: str = "tpch", timeout: int = 30):
    """Execute SQL via raw socket. Returns (ok, stderr)."""
    import threading
    key = (host, port, user)
    try:
        if key not in _thread_conns:
            _thread_conns[key] = RawSocket(host, port, user).connect()
        conn = _thread_conns[key]
        ok, out = conn.execute(sql)
        return ok, out
    except socket.timeout:
        return False, "timeout"
    except Exception as e:
        return False, str(e)

'''

import_point = content.find('\\ndef run_mysql')
if import_point == -1:
    print("Could not find run_mysql")
    exit(1)

new_content = content[:import_point] + rawsocket_code + content[import_point:]
with open('scripts/soak/tpch_mixed_soak_driver.py', 'w') as f:
    f.write(new_content)
print("Patched successfully")