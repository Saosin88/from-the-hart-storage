# Build Storage Lambda Functions

Build optimized Lambda binaries for the storage service using feature flags.

Execute both builds:

**HTTP Handler:**
```bash
cd /home/sheldon/Projects/From\ The\ Hart/from-the-hart-storage
cargo build --release --bin bootstrap_http --features http --no-default-features
```

**SQS Handler:**
```bash
cd /home/sheldon/Projects/From\ The\ Hart/from-the-hart-storage
cargo build --release --bin bootstrap_sqs --features sqs --no-default-features
```

After builds complete, report:
1. Both binary sizes (compare to ensure no bloat)
2. Build times
3. Verify no warnings about unused dependencies
4. Confirm feature-gated code compiled correctly
