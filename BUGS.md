# mdstream rendering bug catalog

Battle-test campaign against CommonMark 0.31.2 + GFM. Each bug was verified by running the real release binary and comparing to spec ground truth.

## Already fixed before this catalog

- **F0**: table delimiter row with <3 hyphens (`--:`,`:-:`) failed to promote (regex `-{3,}` -> `-+`). FIXED + tested.

## Known pre-identified (counted, fix pending)

- **K1**: 4-backtick fence closed by inner 3-backtick line (content leak)
- **K2**: closing fence with trailing text treated as closer (content loss)
- **K3**: code span `` ``a ` b`` `` mis-rendered


## Code blocks / fenced / indented / code spans  (16 bugs)

### [001] Indented code blocks are not recognized at all
- **Input** (CommonMark ex.107): `    a simple\n      indented code block\n` (4-space indent)
- **Expected**: a code block; content with the first 4 spaces stripped: `a simple` / `··indented code block`, rendered as code (monospace/boxed, no inline parsing).
- **Actual**: emitted as a plain paragraph keeping the raw leading spaces verbatim: `    a simple\n      indented code block` — no code framing, indentation not normalized.
- **Severity**: high (structure lost; worse, see next bug for inline-parse leakage)

### [002] Inline markdown is parsed inside what should be a (4-space) indented code block
- **Input** (ex.110): `    <a/>\n    *hi*\n\n    - one\n`
- **Expected**: literal code block: `<a/>` / `*hi*` / (blank) / `- one`. Nothing is interpreted.
- **Actual**: `    <a/>\n    hi\n\n  • one` — the `*hi*` was turned into emphasis (asterisks consumed) and `- one` was turned into a bullet `•`. Content is being treated as normal markdown instead of literal code.
- **Severity**: high (content corruption: emphasis markers eaten, list synthesized)

### [003] A fence-looking line indented 4+ spaces wrongly opens a fence and drops its backticks
- **Input** (ex.134): `    ```\n    aaa\n    ```\n` (every line indented 4 spaces)
- **Expected**: this is an *indented* code block whose literal content is `` ``` `` / `aaa` / `` ``` ``.
- **Actual**: `────…\n  1      aaa\n────…` — mdstream stripped the indent, treated the indented `` ``` `` as a real fence opener+closer, and rendered only `    aaa`. The two literal ```` ``` ```` lines are gone.
- **Severity**: high (content loss)

### [004] A closing fence of a different fence char truncates the block
- **Input** (ex.122): `` ```\naaa\n~~~\n```\n `` (backtick fence, a `~~~` line, then real close)
- **Expected**: `~~~` is ordinary content, so the code block is `aaa` / `~~~`.
- **Actual**: `────…\n  1  aaa\n────…\n────…` — only `aaa` is inside; the `~~~` line was treated as a close (and the real `` ``` `` then drew a third bare border). The `~~~` content line is lost. Symmetric failure for `` ``` `` inside a `~~~` fence (ex.123) and ex.139.
- **Severity**: high (content loss)

### [005] Fenced code block inside a block quote is not recognized; fence markers leak
- **Input** (ex.128): `> ```\n> aaa\n\nbbb\n`
- **Expected**: a code block (content `aaa`) nested inside the blockquote, then a paragraph `bbb`.
- **Actual**: `  │ ```\n  │ aaa\n\nbbb` — the `` ``` `` is shown verbatim as quoted text; no code block is formed. The fence sigils leak.
- **Severity**: high (sigil leak + structure lost)

### [006] Multi-line code spans are not recognized; backtick delimiters leak verbatim
- **Input** (ex.335): `` ``\nfoo\nbar··\nbaz\n``\n `` (double-backtick span spanning newlines)
- **Expected**: one code span; newlines collapse to spaces → `<code>foo bar   baz</code>`.
- **Actual**: `` ``\nfoo\nbar  \nbaz\n`` `` — printed verbatim. The `` `` `` delimiters are not consumed and line-folding never happens. Same failure ex.336, ex.337 (single backtick `` `foo   bar \nbaz` `` left literal).
- **Severity**: high (sigil leak + content not rendered as code)

### [007] Multi-backtick inline code-span delimiters are mishandled; surplus backticks leak
- **Input** (ex.339): `` ``foo`bar``\n `` (2-backtick span containing a literal backtick)
- **Expected**: `<code>foo`bar</code>` — content is `` foo`bar ``.
- **Actual**: `` `foobar`` `` — a leading `` ` `` leaks, the internal literal backtick is dropped, and a trailing `` `` `` leaks. Also reproduced with `` see ``a`b`` end `` → `` see `ab`` end ``, and `` x ```a``` y `` → `` x ``a`` y `` (only one backtick of each delimiter consumed).
- **Severity**: high (sigil leak + content loss)

### [008] Literal backticks inside a code span are dropped (content loss)
- **Input** (ex.330): `` ` `` `\n `` (single-backtick span whose content is two backticks)
- **Expected**: `<code>``</code>` — content is exactly `` `` ``.
- **Actual**: `··` (just two spaces; the backticks vanished). Same class: ex.331 `` `··``··` `` expected `` `·``·` `` content but produced only spaces; ex.340 internal `` `` `` dropped.
- **Severity**: high (content loss)

### [009] Info string is not rejected when it contains a backtick (line eaten, following text lost)
- **Input** (ex.138): `` ``` ```\naaa\n `` (an opening fence line that contains another `` ``` ``)
- **Expected**: per CommonMark a `` ` ``-fence info string may not contain a backtick, so this is NOT a code block: `<p><code> </code>\naaa</p>`.
- **Actual**: `── ``` ──…\n  1  aaa` — mdstream opened a fenced block titling it `` ``` `` and swallowed `aaa` as code. Same with ex.145 (`` ``` aa ``` `` → opens fence, drops `foo`) and ex.347 (`` ```foo`` `` → renders a fence titled `` foo`` `` with no content; expected literal paragraph `` ```foo`` ``).
- **Severity**: high (misparse + content loss)

### [010] Opening-fence indentation is not stripped from content lines
- **Input** (ex.131): ` ```\n aaa\naaa\n```\n` (fence opened with 1 leading space)
- **Expected**: up to N leading spaces (N = fence indent) are removed from each content line → `aaa` / `aaa`.
- **Actual**: `  1   aaa\n  2  aaa` — the first line keeps its leading space (` aaa`) instead of having the 1-space fence indent removed. Worse in ex.133 (3-space fence): content lines retain their absolute indentation rather than having 3 columns stripped (`   aaa` shown as `  aaa`-ish, `    aaa` as `   aaa`), so relative indentation is wrong.
- **Severity**: medium (wrong content whitespace)

### [011] A closing fence indented 4+ spaces still closes the block
- **Input** (ex.137): `` ```\naaa\n    ```\n `` (closing `` ``` `` indented 4 spaces)
- **Expected**: a fence closer may be indented at most 3 spaces; 4 spaces makes it content, so the block is `aaa` / `    ``` ` and is left open to EOF.
- **Actual**: `────…\n  1  aaa\n────…` — the 4-space-indented `` ``` `` was accepted as the closer and the literal `    ``` ` content line was lost.
- **Severity**: medium (content loss)

### [012] Backslash acts as an escape inside a code span (should be literal)
- **Input** (ex.338): `` `foo\`bar`\n ``
- **Expected**: backslashes are literal in code spans, so the first backtick closes the span: `<code>foo\</code>bar`` → visible `foo\` then `bar``.
- **Actual**: `` foo`bar `` — the `\`` was treated as an escaped backtick (backslash removed, backtick kept as content), so the span boundary is wrong and a `\` is lost.
- **Severity**: medium (wrong escape handling, char loss)

### [013] HTML entities are decoded inside code spans (should stay literal)
- **Input**: `` `&amp;`\n ``
- **Expected**: code-span content is literal; `&amp;` stays `&amp;` (corpus ex.343 shows code spans keep `&amp;`/`&lt;` literal).
- **Actual**: `&` — the entity was decoded inside the span. (Confirmed asymmetry: inside *fenced* code, `` ```\n&amp; &lt;\n``` `` is correctly kept literal, so the bug is specific to the code-span path.)
- **Severity**: medium (content altered)

### [014] Stray/mismatched backtick run corrupts surrounding text
- **Input** (ex.349): `` `foo``bar``\n ``
- **Expected**: `` `foo `` is literal (no matching single closer), then `` ``bar`` `` is a code span → `<p>`foo<code>bar</code></p>` i.e. visible `` `foo `` then code `bar`.
- **Actual**: `` foobar` `` — the opening `` ` `` is dropped, `foo` and `bar` are merged, and a stray `` ` `` is appended. Also `` `a`` `` → `` a` `` (leading delimiter dropped, content mangled).
- **Severity**: medium (content reordered/lost)

### [015] Lazy paragraph continuation indented 4 spaces is kept literal, not joined
- **Input** (ex.113): `Foo\n    bar\n\n`
- **Expected**: the indented line is a lazy continuation of the paragraph → `<p>Foo\nbar</p>` (one paragraph, leading spaces removed, soft-wrapped).
- **Actual**: `Foo\n    bar` — mdstream keeps the literal 4-space indent and does not fold the line into the paragraph.
- **Severity**: medium (wrong whitespace/structure)

### [016] Code-block frame width is fixed at 40 columns and ignores COLUMNS; long lines overflow
- **Input**: `` ```\n<60 'Z' chars>\n``` `` with COLUMNS=120
- **Expected**: the box should size to the terminal width; content should fit inside the frame (wrap or clip).
- **Actual**: the top/bottom borders are always exactly 40 dashes regardless of COLUMNS (40/80/120 all give 40), and a 60-char content line runs ~20 columns past the right border. The frame does not respect terminal width.
- **Severity**: low (layout-wrong, no content loss)

TOTAL VERIFIED BUGS: 16


## Inline: emphasis, links, escapes, entities, autolinks  (17 bugs)

### [017] Underscore emphasis `_foo_` is never rendered
- **Input**: `_foo_`
- **Expected**: `foo` in italic (`<em>foo</em>`).
- **Actual**: `_foo_` printed verbatim, no styling. Confirmed across corpus (ex 357 `_foo bar_`, ex 461/463) and adversarial cases. `*`-emphasis works, so `_` is simply unimplemented.
- **Severity**: high

### [018] Underscore strong `__foo__` is never rendered
- **Input**: `__foo__`
- **Expected**: `foo` in bold (`<strong>foo</strong>`).
- **Actual**: `__foo__` printed verbatim, no styling. Corpus ex 382 `__foo bar__`, ex 462. `**` works; `__` is unimplemented.
- **Severity**: high

### [019] Numeric character references not decoded
- **Input**: `&#42; &#x2764; &#35;`
- **Expected**: `*`, a heart, `#` (decimal/hex NCRs decode to the codepoint).
- **Actual**: `&#42; &#x2764; &#35;` printed literally. None of the numeric forms are decoded (corpus "Entity and numeric character references").
- **Severity**: medium

### [020] Most named entity references not decoded
- **Input**: `&copy; &auml; &ouml;`
- **Expected**: `©`, `ä`, `ö`.
- **Actual**: `&copy; &auml; &ouml;` printed literally. Only a tiny set (`&amp;`→`&`, `&lt;`→`<`, `&gt;`→`>`, `&nbsp;`→space) is handled; the full HTML5 entity table is not.
- **Severity**: medium

### [021] Email autolinks not recognized
- **Input**: `<foo@bar.example.com>`
- **Expected**: a styled link with text `foo@bar.example.com` (`<a href="mailto:...">`).
- **Actual**: `<foo@bar.example.com>` printed literally, angle brackets and all. No link styling.
- **Severity**: medium

### [022] Non-http(s) URI autolinks not recognized
- **Input**: `<irc://foo.bar:2233/baz>` and `<mailto:foo@bar.example.com>`
- **Expected**: styled autolinks (any valid scheme is an autolink in CommonMark).
- **Actual**: both printed literally with angle brackets. Only `http`/`https` schemes are styled; other valid schemes leak the raw `<...>` text.
- **Severity**: medium

### [023] Hard line break via two trailing spaces not honored
- **Input**: `foo  \nbaz` (two spaces before newline)
- **Expected**: line break between `foo` and `baz`, trailing spaces removed (`foo<br />baz`).
- **Actual**: `foo  ` retains the two trailing spaces and there is no hard-break semantics; output is `foo  \nbaz`. Corpus ex 633/635/636.
- **Severity**: high

### [024] Hard line break via trailing backslash not honored
- **Input**: `foo\\\nbar` (backslash before newline)
- **Expected**: line break, backslash removed (`foo<br />bar`).
- **Actual**: the backslash is printed literally: `foo\` then `bar`. Corpus ex 634/637. The backslash-newline is neither a break nor an escape.
- **Severity**: high

### [025] Backslash escapes of many ASCII punctuation chars not honored
- **Input**: `\$ \% \& \, \/ \: \; \< \= \? \@ \^`
- **Expected**: the backslash is dropped, leaving `$ % & , / : ; < = ? @ ^` (all ASCII punctuation is escapable).
- **Actual**: backslash is kept: output `\$ \% \& \, \/ \: \; \< \= \? \@ \^`. Corpus ex 12 confirms: many escapes (`\$ \% \& \, \. \/ \: \; \< \= \? \@ \^`) leak the backslash. Only a subset (`\! \" \# \' \( \) \* \+ \- \[ \] \_ \` \{ \| \} \~`) is handled.
- **Severity**: high

### [026] Inline markup inside link text is not parsed
- **Input**: `[**bold link**](u)` (also `[*foo*](u)`, `[foo *bar*](u)`)
- **Expected**: link text rendered with inline styling, e.g. `bold link` in bold; the `**`/`*` sigils consumed.
- **Actual**: link text is emitted verbatim: `**bold link** (u)`, `*foo* (u)`. Emphasis/strong/code inside `[...]` is never processed, so sigils leak into the visible link label.
- **Severity**: medium

### [027] `*foo**bar*` loses the literal `**` (text dropped)
- **Input**: `*foo**bar*`
- **Expected**: `<em>foo**bar</em>` — the inner `**` is literal text inside the emphasis.
- **Actual**: visible text is `foobar` (italic); the `**` characters are deleted entirely. Pure text loss.
- **Severity**: high

### [028] `foo *****` collapses five asterisks to one (text loss)
- **Input**: `foo *****`
- **Expected**: literal `foo *****` (a run that cannot open emphasis stays literal).
- **Actual**: visible text is `foo *` — four of the five asterisks vanish and the survivor is wrongly bolded. Distinct from the known `foo******bar...` leak.
- **Severity**: high

### [029] `a * foo bar*` wrongly emphasized and both delimiters dropped
- **Input**: `a * foo bar*`
- **Expected**: literal `a * foo bar*` (the first `*` is followed by a space so it cannot open emphasis). Corpus ex 351.
- **Actual**: ` foo bar` is italicized and both `*` characters are deleted, yielding `a  foo bar`. Spurious emphasis plus sigil loss.
- **Severity**: medium

### [030] `*(*foo)` wrongly emphasized, asterisks dropped
- **Input**: `*(*foo)`
- **Expected**: literal `*(*foo)` (corpus ex 368).
- **Actual**: `(` is italicized and both `*` deleted → visible `(foo)`. Wrong emphasis boundary and text loss.
- **Severity**: medium

### [031] Trailing-space delimiter run wrongly opens/closes emphasis
- **Input**: `*foo bar *` and `**foo bar **`
- **Expected**: literal, no emphasis (a `*`/`**` preceded by whitespace cannot close). Corpus ex 366/391.
- **Actual**: `foo bar ` is italicized/bolded and the closing `*`/`**` is consumed, dropping the delimiter and adding spurious styling.
- **Severity**: medium

### [032] Unclosed `**`/`*` run is wrongly styled and leaks trailing sigils
- **Input**: `**foo bar baz` (and `*foo bar baz`)
- **Expected**: literal `**foo bar baz` — an unmatched opener stays literal. Corpus ex 471/472.
- **Actual**: the text is bolded/italicized AND a stray `**`/`*` is appended at the end: `foo bar baz**`. Both spurious emphasis and a leaked closing delimiter.
- **Severity**: medium

### [033] Mismatched nested delimiters drop characters
- **Input**: `**a*b**c*`
- **Expected**: `<strong>a<em>b</em></strong>c*`-style structure preserving every character (the trailing `*` is literal).
- **Actual**: visible `abc` — the trailing literal `*` is lost and the run `foo *****bar*****baz` similarly renders `barbaz` (sigils gone, but text-bearing chars also re-grouped incorrectly). General mismatch handling silently eats delimiter characters that CommonMark keeps as literal text.
- **Severity**: medium

TOTAL VERIFIED BUGS: 17


## Lists (ordered, unordered, nesting, items)  (16 bugs)

### [034] Ordered list with `)` delimiter not recognized at all
- **Input**: `1) foo\n2) bar`
- **Expected**: `<ol><li>foo</li><li>bar</li></ol>` (CM ex.302/296 — `)` is a valid ordered delimiter).
- **Actual**: `1) foo` / `2) bar` rendered as a literal paragraph; no list, no bullets, markers leak verbatim. Same failure for `10) foo` (ex.296/297) and `3) baz` standalone.
- **Severity**: high

### [035] Nested ordered lists use hierarchical "1.1 / 1.1.1" numbering instead of restarting
- **Input**: `1. a\n   1. b\n      1. c\n         1. d`
- **Expected**: each nested `<ol>` restarts at its own marker; CM renders inner items as `1.`, `1.`, `1.` (each level a fresh `<ol>` starting at 1).
- **Actual**: prints `1. a` then `1.1 b`, `1.1.1 c`, `1.1.1.1 d` — invents a dotted multi-level outline scheme that does not exist in CommonMark, and drops the trailing `.` on nested items.
- **Severity**: high

### [036] List items indented 1-3 spaces are wrongly treated as deeper nesting
- **Input**: `- a\n - b\n  - c\n   - d\n  - e\n - f\n- g`
- **Expected** (CM ex.310): all seven are siblings of ONE flat list (`<ul><li>a</li>...<li>g</li></ul>`); a leading 1-3 spaces does not nest.
- **Actual**: builds a descending/ascending staircase — `• a`, `◦ b`, `▪ c`, `‣ d`, then back to `▪ e`, `◦ f`, `• g` — treating each indent step as a new nesting level. Same wrong staircase for ex.295 (`- foo\n - bar\n  - baz\n   - boo`, expected 4 flat siblings) and ex.312.
- **Severity**: high

### [037] ATX heading inside a list item is not rendered (leaks literal `#`)
- **Input**: `- # Foo\n- Bar`
- **Expected** (CM ex.300): `<li><h1>Foo</h1></li>` — the item contains a level-1 heading.
- **Actual**: prints `• # Foo` — the `#` is emitted literally, no heading styling/structure.
- **Severity**: high

### [038] Setext underline under a list item produces a horizontal rule and breaks the item
- **Input**: `- Bar\n  ---\n  baz`
- **Expected** (CM ex.300): `<li><h2>Bar</h2>baz</li>` — `---` makes "Bar" an h2 inside the item.
- **Actual**: prints `• Bar`, then a full-width `────` horizontal rule, then `baz` dedented out of the list. The item is torn apart and the heading is lost.
- **Severity**: high

### [039] Blockquote inside a list item is not rendered (leaks literal `>`)
- **Input**: `- > quoted`
- **Expected**: `<li><blockquote><p>quoted</p></blockquote></li>`.
- **Actual**: prints `• > quoted` — the `>` leaks as literal text; no blockquote block is produced inside the item. Also seen in ex.260 (`>>- one` ... `> two` → inner `> two` leaks as `> two`).
- **Severity**: high

### [040] Nested blockquote-in-ordered-item loses structure and lazy continuation
- **Input**: `> 1. > Blockquote\ncontinued here.`
- **Expected** (CM ex.292): outer quote > ordered list > inner quote containing "Blockquote\ncontinued here." (lazy line folded into the inner blockquote paragraph).
- **Actual**: prints `│   1. > Blockquote` (inner `>` leaks literal) and the second line `continued here.` is dedented completely out of all three containers to column 0.
- **Severity**: high

### [041] Empty unordered list item leaks the literal marker
- **Input**: `- foo\n-\n- bar`
- **Expected** (CM ex.281): `<ul><li>foo</li><li></li><li>bar</li></ul>` — three items, the middle one empty.
- **Actual**: `• foo`, then a bare `-` at column 0 (marker leaks, item not recognized), then `• bar`. The list is split and the empty item is lost. (Note: `- foo\n-   \n- bar` with trailing spaces DOES work, so it is specifically the no-trailing-space empty item that breaks.)
- **Severity**: medium

### [042] Empty ordered list item leaks the literal marker
- **Input**: `1. foo\n2.\n3. bar`
- **Expected** (CM ex.283): `<ol><li>foo</li><li></li><li>bar</li></ol>`.
- **Actual**: `1. foo`, then a bare `2.` at column 0 (leaked, breaks the list), then `3. bar`.
- **Severity**: medium

### [043] Multiple list markers on one line are not recognized (markers leak)
- **Input**: `- - foo`  (also `- - - foo`, `1. - 2. foo`)
- **Expected** (CM ex.298/299): nested lists — `<ul><li><ul><li>foo</li></ul></li></ul>`; for `1. - 2. foo` an `<ol>`>`<ul>`>`<ol start=2>` chain.
- **Actual**: prints `• - foo` (and `• - - foo`, `1. - 2. foo`) — only the first marker is consumed, the rest leak as literal text and no nesting is built.
- **Severity**: medium

### [044] Ordered list with start != 1 wrongly interrupts a paragraph
- **Input**: `The number of windows in my house is\n14.  The number of doors is 6.`
- **Expected** (CM ex.304): a single `<p>` — an ordered list starting at a number other than 1 must NOT interrupt a paragraph, so this is one paragraph.
- **Actual**: prints the first line as text, then `  14. The number of doors is 6.` as a list item, splitting the paragraph and renumbering. The text "14." should stay inline in the paragraph.
- **Severity**: medium

### [045] Ordered marker with 10+ digits wrongly treated as a list
- **Input**: `1234567890. not ok`
- **Expected** (CM ex.266): `<p>1234567890. not ok</p>` — CM caps ordered start at 9 digits, so this is a plain paragraph.
- **Actual**: rendered as a list item (`1234567890. not ok` with list indentation), accepting a marker that exceeds the 9-digit limit.
- **Severity**: medium

### [046] Changing the bullet marker does not start a new list
- **Input**: `- a\n- b\n+ c\n+ d`  (also `- foo\n+ bar`, `* foo\n- bar`)
- **Expected** (CM ex.301): a marker change (`-` vs `+` vs `*`) ends one list and begins another (`<ul>a,b</ul><ul>c,d</ul>`). Same for ordered delimiter change `.`→`)` (CM ex.302: `<ol>foo,bar</ol><ol start=3>baz</ol>`).
- **Actual**: all four become one continuous list with identical `•` bullets (`• a • b • c • d`); the marker switch is silently normalized away, so the two-list boundary is invisible.
- **Severity**: low

### [047] Ordered list numbers are taken verbatim from source instead of CM-sequential
- **Input**: `1. one\n3. two\n5. three`
- **Expected** (CM): list starts at first marker (1) and the rendered numbers increment sequentially `1. 2. 3.` regardless of the source numbers.
- **Actual**: prints `1. one`, `3. two`, `5. three` — it echoes the raw source numbers rather than producing CM's sequential numbering.
- **Severity**: low

### [048] Leading-zero ordered start is not normalized
- **Input**: `003. ok\n004. two`
- **Expected** (CM ex.268): `<ol start="3">` — leading zeros stripped, renders as `3.`.
- **Actual**: prints `003. ok` / `004. two` — the literal zero-padded markers are kept verbatim.
- **Severity**: low

### [049] Lazy continuation line under an ordered item is dedented out of the list
- **Input**: `  1.  A paragraph\nwith two lines.`
- **Expected** (CM ex.290): `<ol><li><p>A paragraph\nwith two lines.</p></li></ol>` — the unindented "with two lines." is lazily folded into the item's paragraph.
- **Actual**: prints `1. A paragraph` indented as an item, then `with two lines.` flush at column 0, falling out of the list item instead of being part of its paragraph.
- **Severity**: medium

TOTAL VERIFIED BUGS: 16


## Headings, thematic breaks, paragraphs, blanks, wrap, unicode width  (16 bugs)

### [050] Setext `---` heading not recognized; rendered as plain text + full-width thematic break
- **Input**: `Foo\n---\nbar\n` (CommonMark ex59); also ex80 (2nd block), ex83, ex86
- **Expected**: `<h2>Foo</h2>` then `<p>bar</p>` (the `---` under a paragraph line is a setext H2 underline)
- **Actual**: `Foo` printed as an ordinary paragraph, then a 40-cell `────…` horizontal rule, then `bar`. The line is always treated as a thematic break, never as a heading underline. `Foo` gets no heading style.
- **Severity**: high

### [051] Thematic-break / heading-rule width is hardcoded to 40 cells and ignores terminal width
- **Input**: `***\n` rendered at COLUMNS/TTY widths 20, 40, 100, 120
- **Expected**: an `<hr />` should span the live terminal width (or at least never exceed it)
- **Actual**: always emits exactly 40 `─`. At width 100/120 the rule is far too short; at width 20 the 40-cell rule overflows and wraps onto a second line. Verified identical 40-cell output under a real PTY at every tested width.
- **Severity**: high

### [052] `*-*` (emphasis) mis-detected as a thematic break
- **Input**: ` *-*\n` (CommonMark ex56)
- **Expected**: `<p><em>-</em></p>` (emphasis around a hyphen)
- **Actual**: a full 40-cell `────…` horizontal rule. The content `-` and the emphasis are lost entirely.
- **Severity**: high

### [053] 4-space-indented `***` rendered as a thematic break instead of an indented code block
- **Input**: `    ***\n` (CommonMark ex48)
- **Expected**: `<pre><code>***\n</code></pre>` (4-space indent = code block, literal `***`)
- **Actual**: rendered as a 40-cell horizontal rule.
- **Severity**: medium

### [054] Indented `***` after a paragraph line becomes an HR instead of paragraph continuation
- **Input**: `Foo\n    ***\n` (CommonMark ex49)
- **Expected**: `<p>Foo\n***</p>` (lazy continuation; the indented `***` is paragraph text)
- **Actual**: `Foo` paragraph, then a 40-cell horizontal rule. The `***` is promoted to an HR.
- **Severity**: medium

### [055] ATX heading with 1-3 leading spaces not recognized; marker leaked literally
- **Input**: ` ### foo\n  ## foo\n   # foo\n` (CommonMark ex68)
- **Expected**: `<h3>foo</h3>`, `<h2>foo</h2>`, `<h1>foo</h1>` (up to 3 leading spaces allowed before `#`)
- **Actual**: each line printed verbatim (` ### foo`, etc.) as plain text. No heading recognized, `#` markers leaked. (Note the inconsistency: 1-3 leading spaces ARE accepted before a thematic break, ex47, but rejected before a heading.)
- **Severity**: high

### [056] Content loss in ATX heading `# foo#` (trailing `#` with no preceding space)
- **Input**: `# foo#\n` (CommonMark ex75)
- **Expected**: `<h1>foo#</h1>` (a `#` not preceded by a space is part of the text)
- **Actual**: heading renders just `foo`; the trailing `#` is stripped, so `foo#` becomes `foo`. Content lost.
- **Severity**: high

### [057] Backslash-escaped closing-sequence ATX heading truncates content and leaks the backslash
- **Input**: `### foo \###\n## foo #\##\n# foo \#\n` (CommonMark ex76)
- **Expected**: `<h3>foo ###</h3>`, `<h2>foo ###</h2>`, `<h1>foo #</h1>` (escaped `#` are literal text)
- **Actual**: renders `foo \`, `foo #\`, `foo \` respectively. Everything after the escaped `#` is dropped and the `\` is shown literally. Heavy content loss.
- **Severity**: high

### [058] Inline emphasis and backslash escapes not processed inside ATX headings
- **Input**: `# foo *bar* \*baz\*\n` (CommonMark ex66)
- **Expected**: `<h1>foo <em>bar</em> *baz*</h1>` (`*bar*` italic, `\*baz\*` literal `*baz*`)
- **Actual**: heading text rendered as `foo *bar* \*baz\*` -- the `*` emphasis markers are not applied and the backslash escapes are not removed (both leaked verbatim).
- **Severity**: medium

### [059] Empty ATX headings mishandled; bare `#` leaked as literal text
- **Input**: `## \n#\n### ###\n` (CommonMark ex79)
- **Expected**: `<h2></h2>`, `<h1></h1>`, `<h3></h3>` (all empty headings)
- **Actual**: produces stray blank lines and a literal `#` (the bare `#` line is emitted as text `#` instead of an empty H1). Inconsistent/garbled output.
- **Severity**: medium

### [060] Heading underline rule width counts characters, not display cells (CJK undercount)
- **Input**: `# 中文标题\n` (4 fullwidth CJK chars = 8 display cells)
- **Expected**: underline rule matching the title's display width (8 cells)
- **Actual**: title occupies 8 cells but the `━` underline is only 4 cells long (one per char). Wide glyphs are counted as width 1. The rule is half the needed length and visibly misaligned.
- **Severity**: medium

### [061] Heading underline rule width miscounts emoji (off by display width)
- **Input**: `# Hello 😀 World\n`
- **Expected**: underline matching display width 14 (emoji is 2 cells)
- **Actual**: title display width is 14 cells but the `━` underline is 13 cells (emoji counted as 1 cell). Same char-vs-cell root cause as the CJK case, distinct glyph class.
- **Severity**: medium

### [062] Consecutive blank lines not collapsed
- **Input**: `a\n\n\n\n\n\nb\n` (5 blank lines between two paragraphs)
- **Expected**: paragraphs separated by a single blank line (CommonMark treats any run of blanks as one paragraph break)
- **Actual**: output preserves all 5 blank lines between `a` and `b`. Vertical whitespace is not normalized.
- **Severity**: medium

### [063] Trailing whitespace leaked in paragraph and heading text
- **Input**: `foo   \nbar\n`; also setext `Foo  \n-----\n` (CommonMark ex89)
- **Expected**: trailing spaces stripped -> `foo`, `Foo`
- **Actual**: output retains the trailing spaces verbatim (`foo   `, `Foo  `). Trailing whitespace on a line is not trimmed.
- **Severity**: medium

### [064] No width-aware paragraph wrapping; long lines overflow terminal width
- **Input**: `The quick brown fox ... forever and ever.` (106 chars) at COLUMNS=40 / 80
- **Expected**: the renderer should soft-wrap the paragraph to the terminal width on word boundaries with no mid-word breaks
- **Actual**: mdstream emits the paragraph as one physical line of full length (97/106 cells) regardless of COLUMNS (verified: identical output length at COLUMNS=20 and COLUMNS=200, piped). It performs no wrapping and relies on the terminal's own soft-wrap, which breaks mid-word at the raw cell boundary. No continuation indentation is applied.
- **Severity**: medium

### [065] Setext underline applied despite leading indentation on the content line, content indentation leaked
- **Input**: `   Foo\n---\n` (CommonMark ex84)
- **Expected**: `<h2>Foo</h2>` (up to 3 leading spaces stripped from heading content)
- **Actual**: renders `   Foo` (leading spaces preserved) followed by a 40-cell HR -- combines the "setext not recognized" defect with failure to strip leading indentation from the content line.
- **Severity**: low

---

TOTAL VERIFIED BUGS: 16


## Tables (GFM) and blockquotes  (6 bugs)

### [066] blockquote lazy continuation not supported
- **Input**: `> a\nb`
- **Expected**: both lines in the blockquote (CommonMark lazy continuation)
- **Actual**: `a` quoted (`│ a`), `b` falls out of the quote at column 0
- **Severity**: medium

### [067] heading inside blockquote not formatted (marker leaks)
- **Input**: `> # quoted heading`
- **Expected**: a styled heading inside the quote
- **Actual**: literal `# quoted heading` after the `│`
- **Severity**: medium

### [068] table inside blockquote not rendered (pipes leak)
- **Input**: `> | a |\n> |---|\n> | 1 |`
- **Expected**: a rendered table inside the blockquote
- **Actual**: raw `| a |` / `|---|` / `| 1 |` lines leak verbatim inside the quote
- **Severity**: medium

### [069] fenced code inside blockquote not rendered (``` leaks)
- **Input**: `> ```\n> code\n> ```\n`
- **Expected**: a code block inside the quote
- **Actual**: literal ``` lines leak
- **Severity**: medium

### [070] bold/inline markup in table cell stripped (not styled)
- **Input**: `| a | b |\n|---|---|\n| **x** | y |`
- **Expected**: cell shows "x" in bold
- **Actual**: cell shows plain `x` (markup removed but not applied as style); underscore emphasis `_y_` would instead leak literally
- **Severity**: low

### [071] link in table cell leaks URL
- **Input**: `| a |\n|---|\n| [t](/u) |`
- **Expected**: styled link text "t"
- **Actual**: `t (/u)` — URL shown in cell (consistent with link rendering design, but in a narrow cell this overflows alignment)
- **Severity**: low


## Emphasis flanking + heading inline (lead)  (6 bugs)

### [072] spaced asterisks treated as emphasis (CommonMark flanking violated)
- **Input**: `a * b * c`
- **Expected**: literal `a * b * c` (a `*` preceded/followed by whitespace is NOT a valid emphasis delimiter)
- **Actual**: `a ` + italic ` b ` + ` c` — the `*` chars are consumed
- **Severity**: high (content corruption)

### [073] arithmetic text mangled by emphasis
- **Input**: `2*3 = 6 and 4*5 = 20`
- **Expected**: literal text (no emphasis; per flanking rules these `*` are not openers/closers because of surrounding context, and even loosely most renderers leave single `*` between digits alone)
- **Actual**: `2` + italic `3 = 6 and 4` + `5 = 20` — math becomes italic gibberish
- **Severity**: high (extremely common in LLM/technical output)

### [074] heading inline markup not formatted (and leaks)
- **Input**: `# Title with \`code\``
- **Expected**: heading where `code` is rendered as an inline code span
- **Actual**: literal backticks shown: `Title with \`code\``
- **Severity**: medium

### [075] heading bold/italic not formatted
- **Input**: `# Title **bold**`
- **Expected**: heading with "bold" in bold
- **Actual**: literal `Title **bold**`
- **Severity**: medium

### [076] heading underline length counts markup characters
- **Input**: `# Title **bold**` (h1) or `# Title with \`code\``
- **Expected**: underline width = visible text width
- **Actual**: `━` underline counts the literal `**`/backtick chars, so the rule is too long / misaligned
- **Severity**: low

### [077] single spaced `*` between letters still italicizes
- **Input**: `compute 3*x*y now`
- **Expected**: depends, but `3*x*y` with no spaces: CommonMark makes `x` emphasized only if flanking holds; here mdstream yields `3`+italic`x`+`y`. Borderline, but combined with the math case shows the regex has no flanking awareness at all.
- **Severity**: medium


## Nested inline style RESET-clobber (lead)  (5 bugs)

### [078] bold inside italic loses italic after the bold span
- **Input**: `*a **b** c*`
- **Expected**: italic "a ", bold-italic "b", italic " c"
- **Actual**: italic "a ", bold "b", then RESET kills italic so " c" is unstyled
- **Severity**: high

### [079] italic inside bold loses bold after the italic span
- **Input**: `**a *b* c**`
- **Expected**: bold "a ", bold-italic "b", bold " c"
- **Actual**: " c" loses bold (RESET after inner italic)
- **Severity**: high

### [080] code span inside emphasis loses emphasis after the code
- **Input**: `*a \`b\` c*`
- **Expected**: italic "a ", code "b", italic " c"
- **Actual**: " c" loses italic (RESET after code span)
- **Severity**: high

### [081] strikethrough inside bold loses bold after the strike
- **Input**: `**a ~~b~~ c**`
- **Expected**: bold "a ", bold-strike "b", bold " c"
- **Actual**: " c" loses bold
- **Severity**: medium

### [082] link inside bold loses bold after the link
- **Input**: `**see [t](/u) end**`
- **Expected**: bold "see ", styled link, bold " end"
- **Actual**: " end" loses bold (RESET after link render)
- **Severity**: medium


## Link reference definitions, tabs, indentation (lead)  (6 bugs)

### [083] tab-indented line not treated as code block
- **Input**: `\tcode with tab`
- **Expected**: code block (a leading tab counts as 4 spaces of indent)
- **Actual**: rendered as plain paragraph with literal tab
- **Severity**: medium

### [084] link reference definition leaks as literal text
- **Input**: `[foo]: /url "t"\n\nuse [foo]`
- **Expected**: the definition line is consumed (not shown); `[foo]` resolves to a link
- **Actual**: `[foo]: /url "t"` printed verbatim, and `use [foo]` keeps literal brackets
- **Severity**: high

### [085] shortcut reference link not resolved
- **Input**: `[foo]\n\n[foo]: /u`
- **Expected**: styled link "foo"
- **Actual**: literal `[foo]`
- **Severity**: medium

### [086] collapsed reference link `[foo][]` not resolved
- **Input**: `[foo][]\n\n[foo]: /u`
- **Expected**: styled link "foo"
- **Actual**: literal `[foo][]`
- **Severity**: medium

### [087] full reference link `[text][ref]` not resolved
- **Input**: `[text][ref]\n\n[ref]: /u`
- **Expected**: styled link "text"
- **Actual**: literal `[text][ref]`
- **Severity**: medium

### [088] leading spaces (1-3) in paragraph not stripped
- **Input**: `   indented para`
- **Expected**: `indented para` (CommonMark strips up to 3 leading spaces)
- **Actual**: `   indented para` (spaces kept)
- **Severity**: low


## Misc: underscore emphasis, entities, link titles, indented code (lead)  (10 bugs)

### [089] underscore emphasis `_x_` not supported
- **Input**: `_italic_`
- **Expected**: italic "italic" (CommonMark emphasis with `_`)
- **Actual**: literal `_italic_`, no styling
- **Severity**: high (common markdown, totally unstyled)

### [090] underscore strong `__x__` not supported
- **Input**: `__bold__`
- **Expected**: bold "bold"
- **Actual**: literal `__bold__`
- **Severity**: high

### [091] underscore strong+emphasis `___x___` not supported
- **Input**: `___x___`
- **Expected**: bold italic "x"
- **Actual**: literal `___x___`
- **Severity**: high

### [092] link title leaks into output
- **Input**: `[text](/url "title")`
- **Expected**: styled link "text" (url shown without the quoted title), per CommonMark the title is metadata
- **Actual**: `text (/url "title")` — the `"title"` is part of the shown URL
- **Severity**: medium

### [093] numeric character references not decoded
- **Input**: `&#35; and &#x40;`
- **Expected**: `# and @`
- **Actual**: literal `&#35; and &#x40;`
- **Severity**: medium

### [094] named entity `&copy;` (and most named entities) not decoded
- **Input**: `&copy; 2026`
- **Expected**: `© 2026`
- **Actual**: literal `&copy; 2026` (only ~6 entities hardcoded)
- **Severity**: low

### [095] consecutive blank lines not collapsed
- **Input**: `a\n\n\n\n\nb`
- **Expected**: one blank line between paragraphs (CommonMark collapses)
- **Actual**: all blank lines preserved (4 blank rows)
- **Severity**: low

### [096] indented code block (4 spaces) not rendered as code
- **Input**: `    int x = 1;`
- **Expected**: a code block (boxed/styled)
- **Actual**: shown as raw indented text, no code styling
- **Severity**: medium

### [097] code span surrounding-space not trimmed
- **Input**: `` ` x ` ``
- **Expected**: code "x" (CommonMark strips one leading+trailing space when both present)
- **Actual**: code " x " (spaces kept)
- **Severity**: low

### [098] email autolink `<foo@bar.com>` not linked
- **Input**: `<foo@bar.com>`
- **Expected**: styled email autolink
- **Actual**: literal `<foo@bar.com>`
- **Severity**: low


---
TOTAL CATALOGED: 98 (files) + 3 known (K1-K3) + 1 fixed (F0) = 102
