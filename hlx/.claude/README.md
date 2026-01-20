# Claude Code Context System for HLX Development

**Purpose:** Maintain full architectural context across sessions, preventing ramp-up time and repeated mistakes.

---

## 🚀 AT SESSION START

**Follow these steps before doing any work:**

1. **Read HLX_DESIGN_SPEC.md** (10 min)
   - Understand what HLX is
   - Review current implementation status
   - Confirm vision and goals

2. **Read ARCHITECTURAL_DECISIONS.md** (15 min)
   - Understand WHY each decision was made
   - Review open questions
   - See what's blocked on decisions

3. **Read most recent session breadcrumb** (5 min)
   - Example: `SESSION_BREADCRUMBS/session_2026-01-19.md`
   - Understand what was accomplished
   - See what's blocked or needs work
   - Note any pending user decisions

4. **Check for open TODO items**
   - Look for "TODO", "BLOCKED", "PENDING" in breadcrumbs
   - See what the highest priority is
   - Understand what needs testing

**Total startup time:** ~30 minutes to full context

**Prompt to add to Claude's memory:**
```
At the start of each session, read:
1. /.claude/HLX_DESIGN_SPEC.md
2. /.claude/ARCHITECTURAL_DECISIONS.md
3. /.claude/SESSION_BREADCRUMBS/[most recent date].md
```

---

## 📝 AT SESSION END

**Follow these steps before wrapping up:**

1. **Check if session breadcrumb exists**
   - If `session_2026-01-19.md` exists, update it
   - If session is continuing tomorrow, create `session_2026-01-20.md`

2. **Fill in the breadcrumb with:**
   - [ ] Work log for all tasks completed
   - [ ] Key decisions made
   - [ ] Tests run and their status
   - [ ] Files created/modified
   - [ ] Next session priorities
   - [ ] Known blockers
   - [ ] Any decisions needing user input

3. **Update ARCHITECTURAL_DECISIONS.md if needed**
   - Add any new decisions made this session
   - Mark status of decisions
   - Link to relevant code

4. **Update HLX_DESIGN_SPEC.md if needed**
   - Update implementation status
   - Mark completed features
   - Update version if major changes

5. **Note any discoveries**
   - Add to "Questions that emerged"
   - Add to "Rabbit holes we avoided"
   - Document learnings

6. **Verify links and references**
   - Make sure all file paths are correct
   - Make sure all links work
   - Make sure instructions are clear for next session

**Prompt to add to Claude's memory:**
```
At the end of each session, update:
1. /.claude/SESSION_BREADCRUMBS/session_[DATE].md with all work completed
2. /.claude/ARCHITECTURAL_DECISIONS.md if decisions were made
3. /.claude/HLX_DESIGN_SPEC.md if implementation status changed
```

---

## 📚 File Guide

### HLX_DESIGN_SPEC.md
**What:** Master specification of HLX language
**When to read:** Session start, whenever you need to understand vision
**When to update:** Implementation status changes, major features added
**Size:** ~300 lines | **Read time:** 10 minutes

### ARCHITECTURAL_DECISIONS.md
**What:** The "why" behind each design decision
**When to read:** Session start, whenever you need to understand trade-offs
**When to update:** New decisions made, status changes
**Size:** ~400 lines | **Read time:** 15 minutes
**Key sections:**
- Decision 1-7: Core language architecture
- Decision 8-14: Implementation choices
- "Decisions Still Open" for pending questions

### SESSION_BREADCRUMBS_TEMPLATE.md
**What:** Template for daily session tracking
**When to use:** Copy at start of each new day/session
**When to read:** Understand the format before writing breadcrumbs
**Size:** ~120 lines | **Read time:** 5 minutes

### SESSION_BREADCRUMBS/session_YYYY-MM-DD.md
**What:** Daily progress log
**When to read:** Session start (read most recent)
**When to update:** Session end (fill in all sections)
**Size:** Grows during session | **Read time:** 5-10 minutes
**Important sections:**
- Starting Context (what we were doing)
- Next Session Priorities (what to focus on)
- Known blockers (what's stuck)
- Attached Context (user decisions needed)

---

## 🎯 Workflow

```
Monday Session Start
    ↓
Read design spec, decisions, last breadcrumb
    ↓
Work on highest-priority items from "Next Session Priorities"
    ↓
At end: Update breadcrumb with all work done
    ↓
Tuesday Session Start
    ↓
Read updated breadcrumb → Know exactly what to do next
    ↓
No ramp-up time, full context
```

---

## 💡 Why This Works

1. **Session continuity:** Each session knows what the previous one did
2. **Vision alignment:** Design spec prevents drift
3. **Decision memory:** Architectural decisions explain the "why"
4. **Reduced ramp-up:** 30 minutes of reading saves hours of "why did we do this?"
5. **No repeated mistakes:** Decisions document are there for a reason
6. **Easy onboarding:** New people can read the docs and understand the project

---

## 🔄 Continuous Improvement

**This system improves as we use it:**

1. **First few sessions:** We'll discover what information is most valuable
2. **Patterns emerge:** We'll see recurring questions and add sections for them
3. **Context deepens:** More detail accumulates in breadcrumbs
4. **History builds:** By session 30, we'll have complete project history

**Feedback loop:**
- If something is confusing → add clarification to design spec
- If a decision is questioned → clarify rationale in decisions file
- If a mistake repeats → add note to prevent it in breadcrumbs

---

## 🚨 Red Flags That This System Isn't Working

- Claude asks "why did we do X?" when it's documented in ARCHITECTURAL_DECISIONS.md
- Sessions take >30 min to get up to speed
- Decisions are made that contradict documented choices
- Breadcrumbs aren't updated, breaking continuity

**If you see these, improve the docs!**

---

## 📋 Quick Checklist

**Before any work starts:**
- [ ] Read HLX_DESIGN_SPEC.md
- [ ] Read ARCHITECTURAL_DECISIONS.md
- [ ] Read most recent session breadcrumb
- [ ] Understand current priorities
- [ ] Check for blocked items

**Before ending a session:**
- [ ] Fill in session breadcrumb completely
- [ ] Update ARCHITECTURAL_DECISIONS.md if decisions changed
- [ ] Update HLX_DESIGN_SPEC.md if status changed
- [ ] Note next session's starting point clearly
- [ ] List any user decisions needed
- [ ] Verify all links and file paths

---

## 🎓 Example Session Flow

**Session Start (Monday):**
```
1. Read HLX_DESIGN_SPEC.md → Understand HLX is:
   - Procedural language + contract values
   - Dual representation (A/R)
   - Four axioms enforced
   - Written in HLX (self-hosting)

2. Read ARCHITECTURAL_DECISIONS.md → Understand:
   - Why RustD separated (Decision 1)
   - Why procedural (Decision 4)
   - Why pass-by-value arrays (Decision 10)
   - What's still open (HLX-D naming?)

3. Read session_2026-01-19.md (last breadcrumb) → Understand:
   - Yesterday we did: constant tracking, pointer support
   - QEMU waiting on install
   - Next: complete constant fix, boot kernel
   - Pending: user decision on HLX-D naming

4. Start work on highest priority: complete constant tracking fix
```

**Session End (Monday):**
```
1. Update SESSION_BREADCRUMBS/session_2026-01-19.md with:
   - What we worked on (constant tracking fix)
   - Whether it passed tests
   - What's left to do
   - What to pick up on tomorrow

2. Update ARCHITECTURAL_DECISIONS.md:
   - Mark "Decision 7" as status "In Progress"
   - Add discoveries about constants

3. Update HLX_DESIGN_SPEC.md:
   - Mark "Embedded Contract Syntax" as status change
   - Update version if significant

4. At end of file:
   - Note: "QEMU install still needed - ask user to run: sudo pacman -S qemu-system-x86"
   - Note: "Next session: boot kernel + finalize constant support"
```

**Session Start (Tuesday):**
```
1. Read breadcrumb from Monday → "Oh, we were fixing constants"
2. See next priority → "Boot kernel in QEMU"
3. See blocker → "Need user to install QEMU first"
4. No ramp-up time, ready to work immediately
```

---

## 🔐 Maintaining Accuracy

**The documents should be:**
- **Truthful:** Accurately reflect decisions and status
- **Complete:** Don't leave out important details
- **Linked:** Reference code, files, commits
- **Timely:** Updated before session ends
- **Clear:** Anyone reading should understand why

**If a document is wrong:**
1. Fix it immediately
2. Add note about what was wrong
3. Prevent the error from happening again

---

## 📞 Contact & Questions

If something about this system isn't working or doesn't make sense, update the README to clarify it!

---

**System created:** 2026-01-19
**Last reviewed:** 2026-01-19
**Used by:** Claude Code (Claude Haiku 4.5)
**Maintained by:** User + Claude Code collaboration
