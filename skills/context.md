# Skill: typd Project Context Snapshot

**Trigger:** User says "context snapshot", "generate context", "write context", or "save context"

---

## What You Are

You are a context-snapshot agent for the **typd** project — a Wayland-native virtual keyboard + intelligent text suggestion system written in C.

When triggered, you will:
1. Read all relevant project files
2. Understand the current state of the build
3. Write a structured, timestamped Markdown snapshot to `context/YYYY-MM-DD_HH-MM.md`

---

## Step 1 — Read These Files (in order)

Execute the following reads before writing anything:

```bash
# Project structure
find /home/rev/Documents/projects/typd -type f | sort

# meson.build — understand what is wired up
cat /home/rev/Documents/projects/typd/meson.build

# All source files
cat /home/rev/Documents/projects/typd/src/main.c
cat /home/rev/Documents/projects/typd/src/virtual_kbd.c
cat /home/rev/Documents/projects/typd/src/input_method.c
cat /home/rev/Documents/projects/typd/src/popup.c
cat /home/rev/Documents/projects/typd/src/suggestions.c
cat /home/rev/Documents/projects/typd/src/context_detect.c
cat /home/rev/Documents/projects/typd/src/renderer.c
cat /home/rev/Documents/projects/typd/src/config.c

# All headers
cat /home/rev/Documents/projects/typd/include/common.h
cat /home/rev/Documents/projects/typd/include/virtual_kbd.h
cat /home/rev/Documents/projects/typd/include/input_method.h
cat /home/rev/Documents/projects/typd/include/popup.h
cat /home/rev/Documents/projects/typd/include/suggestions.h
cat /home/rev/Documents/projects/typd/include/context_detect.h
cat /home/rev/Documents/projects/typd/include/renderer.h
cat /home/rev/Documents/projects/typd/include/config.h

# Test files
cat /home/rev/Documents/projects/typd/tests/test_trie.c
cat /home/rev/Documents/projects/typd/tests/test_bktree.c
cat /home/rev/Documents/projects/typd/tests/test_context.c

# Git log for recent commits
git -C /home/rev/Documents/projects/typd log --oneline -10

# Protocols fetched
ls /home/rev/Documents/projects/typd/protocols/

# Data files present
ls /home/rev/Documents/projects/typd/data/
```

---

## Step 2 — Analyse and Determine

From your reads, determine:

- **Which phase** of the project plan is currently active (Phase 01–10)
- **Which tasks within that phase** are done (non-empty implementations) vs pending (empty stubs)
- **What compiles** — does `meson.build` reference real sources with real symbols?
- **What is the next concrete task** the developer needs to do
- **Any known issues, TODOs, or FIXMEs** found in code

Use the phase definitions from `docs/typd-PRD.md` as your reference for what belongs to each phase.

---

## Step 3 — Create the Context File

```bash
mkdir -p /home/rev/Documents/projects/typd/context
```

Filename format: `YYYY-MM-DD_HH-MM.md`  
Use the current system time.

Write the file at:
```
/home/rev/Documents/projects/typd/context/YYYY-MM-DD_HH-MM.md
```

---

## Step 4 — Context File Template

Fill every section. Do NOT leave any section empty. If something is unknown, say "not yet implemented" — never skip.

```markdown
# typd — Context Snapshot
**Date:** YYYY-MM-DD HH:MM
**Project:** typd — Wayland virtual keyboard + suggestion system (C / Wayland)
**Repo:** https://github.com/revanthlol/typd

---

## Current Phase
**Phase XX — [Phase Name]**
Status: [Not Started | In Progress | Complete]

---

## Files State

### Implemented (non-empty, has real logic)
- `src/example.c` — [one-line description of what it does]

### Stubs (empty or skeleton only)
- `src/example.c` — pending

### Headers
- `include/example.h` — [defines what structs/functions]

### Build (meson.build)
- Sources wired: [list]
- Dependencies declared: [list]
- Protocol XMLs scanned: [list or "none yet"]

---

## Protocols

| Protocol | XML Present | Header Generated | Bound in Code |
|----------|-------------|-----------------|---------------|
| wlr-layer-shell-unstable-v1 | ✅/❌ | ✅/❌ | ✅/❌ |
| zwp_virtual_keyboard_v1 | ✅/❌ | ✅/❌ | ✅/❌ |
| zwp_input_method_v2 | ✅/❌ | ✅/❌ | ✅/❌ |
| zwp_input_popup_surface_v2 | ✅/❌ | ✅/❌ | ✅/❌ |
| zwp_text_input_v3 | ✅/❌ | ✅/❌ | ✅/❌ |

---

## Suggestion Engine

| Component | Status | Notes |
|-----------|--------|-------|
| Trie (prefix autocomplete) | ✅/❌/🔧 | |
| BK-Tree (fuzzy correction) | ✅/❌/🔧 | |
| words.freq loaded | ✅/❌ | |

---

## Recent Git Commits
[paste last 5 from git log --oneline]

---

## Known Issues / TODOs
- [list any FIXMEs, TODOs, or obvious gaps found in code]
- or: "None found"

---

## Next Task
**What to implement next (be specific):**

> [One paragraph. Name the exact file, function, and behavior to implement.
>  Reference the PRD section or phase task number.]

---

## Agent Feed Summary
> This section is a compressed, information-dense block for feeding to another LLM.
> Write it as a single paragraph with no fluff.

typd is a C11 Wayland-native virtual keyboard + suggestion daemon. 
Current phase: [XX — name]. 
Implemented: [comma list of done files/functions]. 
Pending: [comma list of stubs]. 
meson.build wires: [sources]. 
Protocols bound: [list]. 
Next task: [one sentence exact task]. 
Constraints: single binary, no GTK/Qt, Cairo rendering, wlroots compositors only, target <2MB stripped <15MB RAM.
```

---

## Step 5 — Commit the Context File

```bash
cd /home/rev/Documents/projects/typd
git add context/
git commit -m "docs: add context snapshot YYYY-MM-DD_HH-MM

- Phase XX status captured
- [one line summary of what is done]
- Next: [one line of next task]"
```

---

## Rules

- **Never fabricate** function names, file contents, or protocol status — read the actual files
- **Never skip** the "Agent Feed Summary" section — it is the most important block for LLM handoff
- **Always use real timestamps** from system time, not hardcoded dates
- If a file is empty (0 bytes or only a comment), mark it as **stub**
- If `meson.build` does not compile yet, say so explicitly in the Build section
- This file is the **single source of truth** for any LLM picking up this project cold

---

## Skill Metadata
**Project:** typd  
**Created:** 2025-05-01  
**Triggered by:** "context snapshot" | "generate context" | "write context" | "save context"  
**Output:** `context/YYYY-MM-DD_HH-MM.md`
