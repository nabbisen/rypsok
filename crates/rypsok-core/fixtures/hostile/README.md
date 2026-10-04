# Hostile corpus

Inputs that try to break the content boundary or exhaust resources
(REQ-TEST-006, REQ-TEST-007): closing markers, Markdown in titles, hidden
elements, zero-width and tag characters, bidirectional controls,
instruction-like text, deep nesting, oversized bodies.

Each file is named `<threat>-<short description>.<ext>` and is listed in the
corpus test with the requirement it exercises. The corpus grows with every
incident and every fuzz finding; nothing is removed.
