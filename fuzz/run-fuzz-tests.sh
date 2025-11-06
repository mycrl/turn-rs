#!/bin/bash
# Run all fuzz tests for turn-server
# Usage: ./run-fuzz-tests.sh [duration_per_target_in_seconds]

set -e

DURATION=${1:-60}  # Default to 60 seconds per target
FUZZ_TARGETS=(
    "fuzz_stun_decoder"
    "fuzz_stun_message"
    "fuzz_channel_data"
    "fuzz_stun_attributes"
)

echo "========================================="
echo "TURN Server Fuzz Testing Suite"
echo "========================================="
echo "Duration per target: ${DURATION} seconds"
echo ""

# Check if cargo-fuzz is installed
if ! command -v cargo-fuzz &> /dev/null; then
    echo "Error: cargo-fuzz is not installed"
    echo "Install it with: cargo install cargo-fuzz"
    exit 1
fi

# Seed corpus with test samples if available
echo "Seeding corpus with test samples..."
if [ -d "tests/samples" ]; then
    for target in "${FUZZ_TARGETS[@]}"; do
        corpus_dir="fuzz/corpus/${target}"
        mkdir -p "$corpus_dir"
        
        # Copy any binary test samples
        find tests/samples -type f -name "*.bin" -exec cp {} "$corpus_dir/" 2>/dev/null \; || true
        
        corpus_count=$(find "$corpus_dir" -type f | wc -l)
        echo "  - ${target}: ${corpus_count} seed inputs"
    done
    echo ""
fi

# Run each fuzz target
total_targets=${#FUZZ_TARGETS[@]}
current=0

for target in "${FUZZ_TARGETS[@]}"; do
    current=$((current + 1))
    echo "========================================="
    echo "Running fuzz target [$current/$total_targets]: $target"
    echo "========================================="
    
    # Run with timeout and continue on error
    cargo fuzz run "$target" -- \
        -max_total_time="$DURATION" \
        -rss_limit_mb=2048 \
        -print_final_stats=1 \
        || {
            echo "Warning: Fuzz target $target encountered an issue"
            # Check if crash artifacts were created
            if [ -d "fuzz/artifacts/$target" ] && [ -n "$(ls -A fuzz/artifacts/$target 2>/dev/null)" ]; then
                echo "CRASH DETECTED! Artifacts saved in fuzz/artifacts/$target/"
                ls -lh "fuzz/artifacts/$target/"
            fi
        }
    
    echo ""
done

echo "========================================="
echo "Fuzz Testing Complete"
echo "========================================="
echo ""

# Summary of findings
echo "Summary:"
for target in "${FUZZ_TARGETS[@]}"; do
    artifact_dir="fuzz/artifacts/$target"
    if [ -d "$artifact_dir" ] && [ -n "$(ls -A $artifact_dir 2>/dev/null)" ]; then
        count=$(find "$artifact_dir" -type f | wc -l)
        echo "  - $target: $count crash(es) found in $artifact_dir"
    else
        echo "  - $target: No crashes detected"
    fi
done

echo ""
echo "To reproduce a crash, run:"
echo "  cargo fuzz run <target> fuzz/artifacts/<target>/<crash-file>"
