# File Reference Implementation (`@` syntax)

## Overview

When a user types `@path/to/file` in a message, opencode automatically reads the file and includes its content in the prompt sent to the model. This document describes how the `@` syntax is implemented, what data is sent to the model, and the limitations.

## 1. Regex Matching

The `@` reference detection is performed by a regular expression defined in `packages/opencode/src/config/markdown.ts:6`:

```ts
export const FILE_REGEX = /(?<![\w`])@(\.?[^\s`,.]*(?:\.[^\s`,.]+)*)/g
```

**Pattern breakdown:**
- `(?<![\w\`])` – Negative lookbehind to avoid matching email addresses or backtick‑quoted references.
- `@` – The literal `@` character.
- `(\.?[^\s\`,.]*(?:\.[^\s\`,.]+)*)` – Captures the file path:
  - `\.?` – Optional leading dot (for hidden files like `.gitignore`).
  - `[^\s\`,.]*` – Characters that are not whitespace, comma, period, or backtick.
  - `(?:\.[^\s\`,.]+)*` – Allows dots inside the path (e.g., file extensions).

The regex matches patterns like `@src/main.ts`, `@./config.json`, `@~/bin/script`, but not `user@example.com` or `` `@file` ``.

## 2. Processing Pipeline

### 2.1 `resolvePromptParts` (`packages/opencode/src/session/prompt.ts:187`)

After the user message is received, `resolvePromptParts` scans the template for `@` references using `ConfigMarkdown.files()`. For each match:

1. The captured path is resolved:
   - If it starts with `~/`, it is expanded to the user’s home directory.
   - Otherwise, it is resolved relative to the current worktree (`Instance.worktree`).
2. The filesystem is stat’ed to check existence and type.
3. If the path does **not** exist, the system checks whether an agent with that name exists (allowing `@agent` references).
4. If the path is a directory, a file part with `mime: "application/x-directory"` is created.
5. If the path is a regular file, a file part with `mime: "text/plain"` is created (regardless of the actual file type).

The resulting parts are added to the user message’s part list.

### 2.2 `createUserMessage` – File Handling (`packages/opencode/src/session/prompt.ts:970‑1258`)

Each file part is processed according to its protocol and MIME type.

#### **`file:` protocol with `mime: "text/plain"`**

1. The file path is extracted from the `file:` URL.
2. If the URL contains `start`/`end` query parameters (e.g., from LSP symbol searches), they are used as `offset`/`limit` for reading a specific line range.
3. The `ReadTool` is invoked with the file path and optional offset/limit.
4. The tool’s execution returns the file content (or an error message).
5. Two synthetic text parts are added to the message:
   - A “Called the Read tool…” statement describing the read operation.
   - The actual file content (or error text).
6. The original file part is also kept (but later ignored when building model messages).

#### **`file:` protocol with `mime: "application/x-directory"`**

1. The `ReadTool` is called with the directory path.
2. The tool returns a formatted listing of directory entries.
3. Synthetic text parts are added for the tool call and the listing.
4. The original directory file part is kept (and later ignored).

#### **Other MIME types (binary files)**

If the file’s MIME type is neither `text/plain` nor `application/x-directory` (e.g., images, PDFs, unknown binary files):

1. The file is read as binary and encoded as a base64 data URL.
2. A synthetic text part stating “Called the Read tool…” is added.
3. A file part with the data‑URL is added (this part will be sent to the model as a media attachment).

#### **`data:` protocol**

Used for already‑embedded content (e.g., from previous tool results). The base64‑encoded data is decoded and added as a synthetic text part.

#### **MCP resources**

If the file part has a `source.type === "resource"`, the system reads the resource via the Model Context Protocol and injects its text or binary content as synthetic parts.

### 2.3 `toModelMessages` – Final Message Assembly (`packages/opencode/src/session/message‑v2.ts:565`)

When converting internal message parts to the format expected by the LLM provider:

- Text parts (including the synthetic ones containing file content) are passed as regular `text` parts.
- File parts with `mime: "text/plain"` or `"application/x-directory"` are **ignored** – their content has already been conveyed via the synthetic text parts.
- File parts with other MIME types (images, PDFs, etc.) are added as `file` parts with `url` and `mediaType`. The provider‑specific SDK handles these as media attachments.

## 3. What Gets Sent to the Model

| File Type | Content Sent to Model | Format |
|-----------|-----------------------|--------|
| Text file (`*.txt`, `*.js`, `*.py`, …) | File content (with line numbers) up to **2000 lines** or **50 KB**, truncated per‑line at **2000 characters**. | Plain text (synthetic text part) |
| Directory | Listing of entries (sorted alphabetically) up to 2000 entries. | Plain text (synthetic text part) |
| Image (PNG, JPEG, etc.) / PDF | Base64‑encoded data URL attached as a media file. | `file` part with `mediaType` |
| Other binary files (`.exe`, `.zip`, …) | Error message: “Cannot read binary file: …” (if the `ReadTool` binary detection triggers). | Plain text (error) |
| Non‑existent path (agent reference) | If an agent with that name exists, an `agent` part is added, followed by a synthetic text prompting the task tool to invoke the subagent. | `agent` part + text part |

### 3.1 ReadTool Limits (`packages/opencode/src/tool/read.ts:13‑15`)

- **Default line limit**: 2000 lines.
- **Maximum line length**: 2000 characters (longer lines are truncated with `…`).
- **Maximum bytes per read**: 50 KB (bytes counted as UTF‑8).
- If the file exceeds these limits, a truncation notice is appended, and the user can use the `offset` parameter to read further sections.

### 3.2 Binary Detection

The `ReadTool` uses a combination of:
- Extension blacklist (`.zip`, `.exe`, `.class`, …).
- Presence of null bytes.
- Percentage of non‑printable characters (>30%).

If a file is detected as binary, the tool throws an error, which surfaces as an error text part in the prompt.

## 4. Limitations

1. **MIME detection is naïve**: `resolvePromptParts` marks every regular file as `text/plain`. Actual MIME type is only determined later when the file is opened (via `Bun.file().type` or binary detection).
2. **Binary file handling**: Only images and PDFs are automatically attached; other binary files cause an error.
3. **No recursive directory expansion**: `@dir` lists only the top‑level entries; subdirectories are not automatically read.
4. **No glob patterns**: The regex does not support wildcards; each reference must be a single path.
5. **Truncation**: Large text files are truncated, which may omit relevant context.

## 5. Example Flow

User message:
```
Please look at @src/main.ts and @docs/plan.md.
```

1. Regex matches `@src/main.ts` and `@docs/plan.md`.
2. `resolvePromptParts` creates two file parts with `file:` URLs.
3. `createUserMessage` calls `ReadTool` for each file.
4. For `src/main.ts` (text file), synthetic text parts are added with its content.
5. For `docs/plan.md` (text file), same treatment.
6. `toModelMessages` discards the `text/plain` file parts, keeping only the synthetic text parts.
7. The final prompt sent to the model contains the full content of both files (within truncation limits).

## 6. References

- `packages/opencode/src/config/markdown.ts:6` – FILE_REGEX
- `packages/opencode/src/session/prompt.ts:187` – resolvePromptParts
- `packages/opencode/src/session/prompt.ts:970‑1258` – createUserMessage file handling
- `packages/opencode/src/session/message‑v2.ts:565` – toModelMessages ignoring text/plain file parts
- `packages/opencode/src/tool/read.ts:13‑15` – ReadTool limits
