# Worklog

A makeshift issue tracker: nested folders of markdown, versioned with the code it
describes. It exists to hold the things a commit message cannot — why an approach was
chosen over the one you would expect, what was tried and abandoned, what the next person
(or the next session) should pick up.

## Structure

```
worklog/
  YYYY-MM-DD-what-were-doing/          an epic, dated from the day it started
    YYYY-MM-DD-NN-name.md              a session's work, or a self-contained task
    YYYY-MM-DD-bigger-task/            a task that needs several files or days
      YYYY-MM-DD-NN-name.md
```

An **epic** is a folder, named for the day it started and what it is about. Anything
inside it is either a single markdown file — one session, or one task — or a folder of
them when a task runs long enough to need its own days.

Dates are the date of the work, not the date of writing. They sort chronologically, which
is the only ordering that matters here.

**Entry files carry a two-digit sequence number**, `YYYY-MM-DD-NN-name.md`, counting from
01 within a day. More than one session lands on the same date often enough that a bare
date stops sorting them, and renaming an entry after the fact breaks any link to it.
Folders do not take a number: an epic or a multi-day task spans dates, so a sequence
within one date would mean nothing.

## `RULES-CHECKLIST.md`

One file at the top of `worklog/`, not inside an epic, because it outlives them.

This project has an unusually sharp notion of "correct": there is a two-page rulebook,
and either the code plays the game it describes or it does not. What the checklist
collects is everything in between — the places where the code had to decide something
**the rulebook does not settle**, or where a number is believed rather than derived.

Concretely, an item belongs here when it is one of:

- **An inference.** The rules are silent or ambiguous and we picked a reading. Say which
  reading, and what a different reading would change.
- **An unverified number.** An expected value, probability or distribution that came out
  of our own solver and has never been reproduced by anything independent. Anything with
  an independent derivation belongs in a test instead — see below.
- **A modelling shortcut.** A place where the solver deliberately solves something
  adjacent to the real game because the two coincide, plus the argument for why they do.
  These are the entries most likely to be quietly wrong later, when a rule changes
  underneath the argument.
- **An untested path.** A branch rare enough that no game has plausibly exercised it.

Add to it whenever a fix rests on an inference rather than a quotation, and say *what to
watch for* rather than only what was assumed. Tick items off as they are closed, and say
what closed them.

**When an item is closed by an independent derivation, the number moves into a test.** A
value reproduced by a separate implementation is no longer a belief, and the checklist is
for beliefs; leaving it here means the next reader re-derives something already settled.

## What goes in an entry

- **TL;DR** — a list of the major things done, one line each. Written so that reading only
  this tells you whether the entry is worth opening.
- **Details** — one section per item: what changed, which commits, which branches, and
  the reasoning that is not visible in the diff. Rulebook citations belong here; so do
  the alternatives that were rejected and why.
- **Carried forward** — the previous entry's open items, each either struck through with
  what closed it or restated as still live. See *Sweeping* below.
- **Next steps** — what is left, what is blocked on what, and any decision waiting on a
  human. Be specific enough that it can be picked up cold.

The bias is toward writing down what you would otherwise have to re-derive: a rule read
three times before it made sense, a discrepancy between two solvers and its resolution, a
design choice whose alternative still looks tempting.

**Quote the rulebook rather than paraphrasing it.** Every rules bug this project has had
so far came from a paraphrase that drifted — a summary written from memory, believed, and
then implemented. The rulebook is two pages. Paste the sentence.

## Sweeping

**Every entry ends by sweeping the previous one's open items**, not by starting a fresh
list. Without this, next-steps sections accumulate: the same four items get restated in
four entries, two of them already done, and nobody can tell which list is current.

The sweep is a *Carried forward* section with two parts:

- **Done since** — struck through, each with what closed it. Keeping the strikethrough
  rather than deleting the line means the next reader can see the item was considered and
  resolved, not dropped.
- **Still live** — restated, not linked to. An item that survives three sweeps unchanged is
  worth a sentence about *why* it keeps surviving; "the web UI has never been opened in a
  real browser" reads differently on its fifth appearance than its first.

Sweep only the entry before yours. It has already swept the one before it, so the chain
carries everything forward without anyone re-reading the whole epic. If an item belongs to
the engine rather than to a session — an inference, an unverified number, an untested edge
case — move it to `RULES-CHECKLIST.md` instead and stop carrying it.
