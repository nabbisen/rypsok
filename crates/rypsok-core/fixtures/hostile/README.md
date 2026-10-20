# Hostile corpus

Inputs that try to break the content boundary or exhaust resources
(REQ-TEST-006, REQ-TEST-007): closing markers, Markdown in titles, hidden
elements, zero-width and tag characters, bidirectional controls,
instruction-like text, deep nesting, oversized bodies.

Each file is named `<threat>-<short description>.<ext>` and is listed in the
corpus test with the requirement it exercises. The corpus grows with every
incident and every fuzz finding; nothing is removed.

## Entries

| File | Threat | Exercised by |
|---|---|---|
| `thr-01-envelope-closing-markers.txt` | A title that tries to close the envelope and forge `outcome` | REQ-SEC-002, REQ-TEST-007 |
| `thr-02-invisible-characters.txt` | Zero-width, tag, bidirectional, BOM, soft hyphen and control characters | REQ-SEC-015 |
| `thr-03-instruction-like.txt` | Instruction-like phrases in several casings | REQ-SEC-003 |
| `thr-04-nfc-edge-cases.txt` | Decomposed and composed forms, Hangul jamo | REQ-CTX-004 |
| `thr-05-japanese-at-bound.txt` | Multi-byte text far beyond the snippet bound | REQ-CTX-006 |
| `thr-06-markup-and-entities.txt` | Script tags, entities that encode controls, escaped brackets | REQ-CTX-004 |
