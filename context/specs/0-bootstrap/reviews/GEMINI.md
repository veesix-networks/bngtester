Here is a review of the bngtester AI workflow documentation and project structure, based on your focus areas:


  1. Internal Consistency
  The workflow is mostly consistent, but there are a few technical discrepancies that could trip up an LLM:
   * Directory Naming: CLAUDE.md states the spec path is context/specs/<issue-slug>/, whereas PROCESS.md repeatedly refers to context/specs/<feature>/. An agent might take <feature> literally and
     create a folder named "feature" instead of using the issue slug.
   * Branch Naming: PROCESS.md hardcodes the branching instruction as git checkout -b feat/<scope>-<description>. However, CLAUDE.md correctly lists multiple commit types (fix, test, refactor,
     etc.). If an agent works on a bug or testing issue, PROCESS.md forces them to use a feat/ branch prefix, which violates standard conventional branch naming.
   * Tracker Creation: PROCESS.md mandates that every spec directory MUST have a README.md status tracker. However, the exact outputs listed for Phase 1: Spec Draft only mention
     IMPLEMENTATION_SPEC.md. Claude might not create the README.md during Phase 1 because it isn't explicitly listed in the Phase 1 outputs.


  2. Gaps in Issue Templates
  Yes, there is a notable gap in the Bug template (bug.yml):
   * Missing "Workflow agents" dropdown: CLAUDE.md notes that while bugs normally skip the spec workflow, "complex" bugs do trigger it. If a human files a complex bug, there is no field in the
     template to specify which agents should review the spec.
   * Missing Scope Boundary: PROCESS.md states that all issues must capture the scope boundary ("what is explicitly NOT part of this issue"). The Bug template lacks this field entirely.


  3. Scope Rule Enforcement
  Excellent. The rule "One feature per PR. One PR per issue" is explicitly, clearly, and forcefully repeated across all three foundational documents (README.md, CLAUDE.md, and PROCESS.md). It is
  unambiguous.


  4. Ambiguities That Could Cause Agent Confusion
   * Phase 6 (Post-Implementation Review) Triggers: Phase 6 is optional, but the issue templates only ask: "Which agents should review the spec?". There is no indicator for whether the user wants
     the optional Phase 6 code review, nor which agents should perform it. An agent finishing Phase 5 won't know whether to stop or prompt the human to run Phase 6.
   * The Handoff Prompting: PROCESS.md says Claude MUST provide ready-to-paste prompts for the next agents after Phase 1. If the human selects "All three" agents, Claude knows to generate a prompt
     for Gemini (Phase 2). But it's unclear who generates the prompt for Codex (Phase 3). Does Claude generate both prompts at the end of Phase 1, or is Gemini supposed to generate the Codex prompt
     at the end of Phase 2?
   * Bug "Workarounds": In the PROCESS.md Severity Scale, a HIGH severity finding is defined as having "workarounds". Agents reviewing code might get confused about whether they are supposed to
     suggest workarounds or fix the code.


  5. README Clarity for First-Time Contributors
  Very clear. The README does an excellent job of setting expectations right away: "This project is fully LLM-driven." It provides clear, actionable steps for a human to clone the repo, open the
  Claude CLI, and run a specific copy-paste command to start the workflow. It successfully hides the complexity of the multi-agent orchestration (PROCESS.md) from the human contributor while still
  linking to it for transparency.