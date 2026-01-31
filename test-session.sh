#!/bin/bash

# Test gidterm session persistence

set -e

echo "🧪 Testing gidterm with session persistence..."
echo ""

# Clean up old test sessions
rm -rf .gidterm/sessions/

# Run gidterm with test file (it should auto-detect)
echo "📂 Running gidterm with test-gid-integration.yml..."
timeout 5 cargo run --quiet -- test-gid-integration.yml || true

echo ""
echo "📊 Checking session files..."
ls -lh .gidterm/sessions/ 2>/dev/null || echo "No sessions directory created yet"

echo ""
if [ -f .gidterm/sessions/latest.json ]; then
    echo "✅ Session file created!"
    echo ""
    echo "📄 Session contents:"
    cat .gidterm/sessions/latest.json | jq '.' 2>/dev/null || cat .gidterm/sessions/latest.json
else
    echo "❌ No session file found"
    exit 1
fi

echo ""
echo "✨ Test complete!"
