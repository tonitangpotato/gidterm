# GidTerm Testing Guide

## 🧪 How to Test

### Test 1: Scheduler Logic (Dry Run)

**What it does**: Tests task dependency resolution without actually running commands

```bash
cd ~/clawd/gidterm
cargo run --example test_scheduler
```

**Expected output**:
```
✅ Loaded graph:
   - Project: AgentVerse MVP Test
   - Tasks: 8

📊 Task Dependency Analysis:
✓ frontend-install [pending] - can start immediately
✓ start-redis [pending] - can start immediately
⏳ frontend-server [pending] - waiting for frontend-install
...

🚀 Execution Simulation:
▶ Round 1: Starting 4 tasks (parallel execution!)
...
✅ All tasks completed!
```

---

### Test 2: Simple Commands Test

**What it does**: Tests with basic shell commands (echo, ls)

```bash
cd ~/clawd/gidterm
cargo run --example test_execution
```

**Expected output**:
```
🚀 Testing GidTerm Task Execution
✅ Loaded 4 tasks

Round 1: Starting 2 tasks
  ⚙  hello started
  ⚙  list-files started
  │  hello: Hello from task 1
  │  list-files: total 48
  │  list-files: drwxr-xr-x  12 potato  staff   384 Jan 31 11:27 .
  ✓  hello completed (exit code: 0)
  
Round 2: Starting 1 tasks
  ⚙  world started
  │  world: World from task 2
  ✓  world completed (exit code: 0)
  ...
  
✅ All tasks completed!
```

---

### Test 3: TUI Dashboard (Visual)

**What it does**: Shows the graphical terminal interface

**⚠️ Must run in a real terminal** (not through me):

```bash
cd ~/clawd/gidterm
cargo run -- simple-test.yml
```

**Expected**: A nice bordered dashboard showing all tasks

**Exit**: Press `q` to quit

---

## 🐛 Troubleshooting

### Problem: "command not found: docker"

**Solution**: The test-graph.yml uses Docker. Use simple-test.yml instead:

```bash
cargo run --example test_execution -- simple-test.yml
```

### Problem: TUI shows "Device not configured"

**Cause**: You're running it through me or a non-TTY environment

**Solution**: Run directly in your Mac terminal (Terminal.app or iTerm2)

### Problem: Tasks not completing

**Check**:
1. Is the command valid? Try running it manually first
2. Check the output - error messages will show

---

## ✅ Success Criteria

You know gidterm is working if:

1. **Scheduler test**: Shows correct dependency order
2. **Execution test**: 
   - Tasks start in correct order
   - Output appears
   - Tasks complete
   - Final "✅ All tasks completed!"
3. **TUI**: Beautiful bordered interface appears

---

## 📋 Next Steps After Testing

Once gidterm works:

1. **Create agentverse.gid.yml** - Task graph for AgentVerse
2. **Use it during development** - Manage all services
3. **Improve as needed** - Add features when you need them

---

## 💡 Quick Test Script

Save this as `quick-test.sh`:

```bash
#!/bin/bash
cd ~/clawd/gidterm

echo "🧪 Test 1: Scheduler Logic"
echo "=========================="
cargo run --quiet --example test_scheduler
echo ""

echo "🧪 Test 2: Simple Execution"
echo "============================"
cargo run --quiet --example test_execution
echo ""

echo "✅ All tests complete!"
echo "Now try the TUI: cargo run -- simple-test.yml"
```

Then run:
```bash
chmod +x ~/clawd/gidterm/quick-test.sh
~/clawd/gidterm/quick-test.sh
```

---

**Ready to test? Start with Test 1!** 🚀
