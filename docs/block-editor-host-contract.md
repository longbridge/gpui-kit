# Block-editor host contract (local delivery)

The feature remains opt-in. `gpui-kit/block-editor` forwards through Component
and Base; apps use the facade, not direct component/base dependencies.

## Layer boundary

Base owns the document/parser, native transactions, input handling and immutable
read caches. Component owns the themed `BlockEditor` wrapper and bounded
scrolling/auto-grow surface. Host domain state (sessions, recognizers, attachment
admission and Send) belongs outside the framework. No OS device/service is
required by the model or the audited headless tests.

## Source and native positions

`source_snapshot()` shares canonical source by monotonic document revision;
selection, focus or repeated paint reads do not serialize again.
`caret_prefix_snapshot()` shares native plain text by document revision and
navigation epoch. Neither cache is a writable source owner. `source()` is a
compatibility owned-string reader; hot host paths should use snapshots.

`capture_caret()` inserts at the head even when a selection exists.
`capture_selection()`, `capture_suffix()` and `capture_leaf_range()` cover native
positions. The last takes UTF-8 ranges in the **caret's native leaf**, never in
canonical Markdown. A captured edit is opaque and pinned to model owner,
revision, expected selection and navigation epoch. `replace_captured()` validates
before mutation, edits plain text without reparsing or running input rules, and
returns an updated cumulative range. Each replacement is one undoable
transaction. Other owners, human edits/navigation, undo or source loads reject
stale captures. Hosts retain window/session/request generations and can call
`invalidate_captures()` on external ownership changes even when text is equal.

Empty paragraph/footer boundaries and bare list items are real document nodes.
Import reconstructs exterior end trivia using primary AST spans, distinguishing
unfinished code-owned LF from a closed fence's exterior blank lines. Nonempty
list terminal spaces/tabs survive export/restore. Code body, language and info
metadata are distinct: filename hints survive while the highlighter sees only
language. Footnotes/unsupported HTML use the existing extension pipeline as
opaque source-bearing atoms, normalizing container prefixes and block boundaries.
Destination/title escaping is shared by link, reference and image writers.

## Submission and overlapping grammar

`SubmitPolicy::Newline` is the normal editor default.
`Enter` emits `SubmitRequested` for plain Enter (and Cmd+Enter on macOS).
Its composer-only Shift+Enter performs structural editing: paragraph/list
continuation, table next row, bottom-row append, or empty-row exit. Ctrl+Enter
inserts a table-cell soft break (`<br>` in source); outside tables it performs
structural editing without submitting. Tab/Shift+Tab retain table traversal.
`Newline` and `ModifiedEnter` keep their standalone bindings: Shift+Enter is a
cell soft break, and the platform secondary Enter exits a table or submits,
respectively. `ModifiedEnter` keeps plain Enter editing. Marked composition and
readonly state prevent submission and Enter edits. After `set_submit_policy`,
the owner must notify the editor to refresh its policy-scoped key context.
The host gives its context/slash picker first refusal before Send.
`with_input_rule_filter()` is a pure, pre-rule host policy for overlapping sigils, not a synthetic key or post-edit rollback.

## Presentation and native geometry

Base `BlockEditor` implements generic `Styled` on the native input root, with
no padding or height defaults. Component adds Medium input-family proportions
of 0.625rem horizontal and 0.5rem vertical padding inside its sole scroll owner.
Auto-grow bounds include both the logical 1.5rem content rows and the insets.
Pointer/selection and IME geometry use the same native frame as the caret.
Culling invalidates heights on content width, rem and effective font/wrapping
metrics, not text colors or decorations. Reflow retains a visible caret or a
pending native reveal without pulling a manually scrolled-away caret into view.

## Clipboard and media

`ClipboardPolicy::Host` emits one `PasteRequested` carrying all clipboard
entries and an opaque `PasteRequest`. Both editor action bindings and native
`EntityInputHandler::paste` converge on this route. The host validates current
request ownership before attachment side effects, then calls `apply_paste()`
with admitted text or `None` for image-only consumption. Requests are once-only;
new requests, source edits, navigation, readonly state or ownership invalidation
fence old resolutions. `PlainText` mode bypasses fragment Markdown parsing.

`ImagePolicy::Placeholder` never calls the image source resolver or `img()` for
standard image atoms: inline-flow atomic fallback text retains selection and
source without implicit HTTP/file/data decoding. `Load` retains ordinary editor
behavior. Hosts handling ACP/native attachments keep those outside the Markdown
document; the policy does not replace explicit attachment admission.

## Evidence and limits

Pure fidelity/transaction/cache tests and headless submit/paste/fake-HTTP tests
exercise these contracts. A 49,890-byte nested source and large code prefix have
pointer-reuse/read timing evidence; this is not frame/FPS evidence. Actual
native IME/device authorization and app-level visuals require user-operated
native bundle validation. No transcript journal/schema change is required.
