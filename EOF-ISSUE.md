# EOF Connection Issue Analysis

## Problem Summary

When running `hybrid_soak` with `server_threads > 0` (thread pool mode), all queries fail with "Protocol error: Unexpected EOF". The issue does not occur with `server_threads=0` (thread-per-connection mode).

## Symptoms

1. 100% of queries return "Unexpected EOF" error
2. Server logs show queries are being executed
3. Server Send-Q accumulates large amounts of data (4-6MB per connection)
4. No "Transaction already in progress" errors (fix #3622 is working)

## Root Cause Hypothesis

When `server_threads > 0`, the server uses a thread pool (`ServerThreadPool`). If the pool's send channel is full (200ms timeout), the connection is silently dropped WITHOUT:
1. Sending an error packet to the client
2. Logging a backpressure warning at warn level

The client then sees the connection close and reports "Unexpected EOF".

## Evidence

### Network Statistics
```
Recv-Q Send-Q  Local Address:Port  Peer Address:PortProcess                                  
5951365 68     127.0.0.1:3397      127.0.0.1:39376  users:(("sqlrustgo-mysql",pid=464673,fd=13))
```
Send-Q of 6MB+ indicates server is trying to send but client isn't receiving.

### Listen Queue Overflow
```
3592 times the listen queue of a socket overflowed
3592 SYNs to LISTEN sockets dropped
```
Server is overloaded with connections.

### Code Analysis

In `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql`:
```rust
match p.send_timeout(job, Duration::from_millis(200)) {
    Ok(()) => {}
    Err(crate::testing::SendTimeoutError::Timeout(returned_job)) => {
        crate::testing::BACKPRESSURE_COUNT
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        tracing::debug!(  // <-- DEBUG level, not visible with --log-level warn
            "worker pool full; rejecting connection from {} \
             (BACKPRESSURE_COUNT incremented)",
            returned_job.addr
        );
    }
    // Connection silently dropped here - no error sent to client
}
```

## Workaround

Use `server_threads=0` to use thread-per-connection mode:
```bash
./target/release/sqlrustgo-mysql-server serve \
  --port 3397 \
  --data-dir /tmp/soak-phase1/data \
  --server-threads 0 \  # Use thread-per-connection
  --max-connections 200 \
  --wal-sync off
```

## Next Steps

1. Fix the thread pool backpressure handling to send error packets to clients
2. Add proper connection queuing with bounded backlog
3. Consider using a proper connection pool library (e.g., r2d2, deadpool)

## References

- Issue #3500: hybrid_soak transaction handling
- PR #3622: Idempotent START TRANSACTION fix (merged)
