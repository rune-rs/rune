ace.define('ace/mode/rune-highlight-rules',
    ["require", "exports", "module", "ace/lib/oop", "ace/mode/text_highlight_rules"],
    function (require, exports, module) {
        "use strict";

        const TextHighlightRules = require("ace/mode/text_highlight_rules").TextHighlightRules;
        const oop = require("ace/lib/oop");

        const stringEscape = /\\(?:[nrt0'"\\]|x[\da-fA-F]{2}|u\{[\da-fA-F]{6}\})/.source;

        const RuneHighlightRules = function () {
            // regexp must not have capturing parentheses. Use (?:) instead.
            // regexps are ordered -> the first match is used

            this.$rules = {
                start:
                    [{
                        token: 'variable.other.source.rune',
                        // `(?![\\\'])` to keep a lifetime name highlighting from continuing one character
                        // past the name. The end `\'` will block this from matching for a character like
                        // `'a'` (it should have character highlighting, not variable highlighting).
                        regex: '\'[a-zA-Z_][a-zA-Z0-9_]*(?![\\\'])'
                    },
                    {
                        token: 'string.quoted.single.source.rune',
                        regex: "'(?:[^'\\\\]|" + stringEscape + ")'"
                    },
                    {
                        token: 'identifier',
                        regex: /r#[a-zA-Z_][a-zA-Z0-9_]*\b/
                    },
                    {
                        stateName: "bracketedComment",
                        onMatch: function (value, currentState, stack) {
                            stack.unshift(this.next, value.length - 1, currentState);
                            return "string.quoted.raw.source.rune";
                        },
                        regex: /r#*"/,
                        next: [
                            {
                                onMatch: function (value, currentState, stack) {
                                    var token = "string.quoted.raw.source.rune";
                                    if (value.length >= stack[1]) {
                                        if (value.length > stack[1])
                                            token = "invalid";
                                        stack.shift();
                                        stack.shift();
                                        this.next = stack.shift();
                                    } else {
                                        this.next = "";
                                    }
                                    return token;
                                },
                                regex: /"#*/,
                                next: "start"
                            }, {
                                defaultToken: "string.quoted.raw.source.rune"
                            }
                        ]
                    },
                    {
                        token: 'string.quoted.double.source.rune',
                        regex: '"',
                        push:
                            [{
                                token: 'string.quoted.double.source.rune',
                                regex: '"',
                                next: 'pop'
                            },
                            {
                                token: 'constant.character.escape.source.rune',
                                regex: stringEscape
                            },
                            { defaultToken: 'string.quoted.double.source.rune' }]
                    },
                    {
                        token: 'string.quoted.template.source.rune',
                        regex: '`',
                        push:
                            [{
                                token: 'string.quoted.template.source.rune',
                                regex: '`',
                                next: 'pop'
                            },
                            {
                                token: 'constant.character.escape.source.rune',
                                regex: stringEscape
                            },
                            { defaultToken: 'string.quoted.template.source.rune' }]
                    },
                    {
                        token: ['keyword.source.rune', 'text', 'entity.name.function.source.rune'],
                        regex: '\\b(fn)(\\s+)((?:r#)?[a-zA-Z_][a-zA-Z0-9_]*)'
                    },
                    { token: 'support.constant', regex: '\\b[a-zA-Z_][\\w\\d]*::' },
                    {
                        token: 'keyword.source.rune',
                        regex: '\\b(?:abstract|alignof|as|async|await|become|box|break|catch|continue|const|crate|default|do|dyn|else|enum|extern|for|final|if|impl|in|let|loop|macro|match|mod|move|mut|offsetof|override|priv|proc|pub|pure|ref|return|self|sizeof|static|struct|super|trait|type|typeof|union|unsafe|unsized|use|virtual|where|while|yield)\\b'
                    },
                    {
                        token: 'storage.type.source.rune',
                        regex: '\\b(?:Self|int|float|unit|char|bool|String|Bytes|GeneratorState|Generator|Future|Option|Result|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|isize|usize)\\b'
                    },
                    { token: 'variable.language.source.rune', regex: '\\bself\\b' },

                    {
                        token: 'comment.line.doc.source.rune',
                        regex: '//!.*$'
                    },
                    {
                        token: 'comment.line.double-dash.source.rune',
                        regex: '//.*$'
                    },
                    {
                        token: 'comment.start.block.source.rune',
                        regex: '/\\*',
                        stateName: 'comment',
                        push:
                            [{
                                token: 'comment.start.block.source.rune',
                                regex: '/\\*',
                                push: 'comment'
                            },
                            {
                                token: 'comment.end.block.source.rune',
                                regex: '\\*/',
                                next: 'pop'
                            },
                            { defaultToken: 'comment.block.source.rune' }]
                    },

                    {
                        token: 'keyword.operator',
                        // `[*/](?![*/])=?` is separated because `//` and `/* */` become comments and must be
                        // guarded against. This states either `*` or `/` may be matched as long as the match
                        // it isn't followed by either of the two. An `=` may be on the end.
                        regex: /\$|[-=]>|[-+%^=!&|<>]=?|[*/](?![*/])=?/
                    },
                    { token: "punctuation.operator", regex: /[?:,;.]/ },
                    { token: "paren.lparen", regex: /[\[({]/ },
                    { token: "paren.rparen", regex: /[\])}]/ },
                    {
                        token: 'constant.language.source.rune',
                        regex: '\\b(?:true|false|Some|None|Ok|Err|Resume|Yield)\\b'
                    },
                    {
                        token: 'meta.preprocessor.source.rune',
                        regex: '\\b\\w\\(\\w\\)*!|#\\[[\\w=\\(\\)_]+\\]\\b'
                    },
                    {
                        token: 'constant.numeric.source.rune',
                        regex: /\b(?:0x[a-fA-F0-9_]+|0o[0-7_]+|0b[01_]+|[0-9][0-9_]*(?!\.))\b/
                    },
                    {
                        token: 'constant.numeric.source.rune',
                        regex: /\b(?:[0-9][0-9_]*)(?:\.[0-9][0-9_]*)?(?:[Ee][+-][0-9][0-9_]*)?\b/
                    }]
            };

            this.normalizeRules();
        };

        RuneHighlightRules.metaData = {
            fileTypes: ['rn'],
            foldingStartMarker: '^.*\\bfn\\s*(\\w+\\s*)?\\([^\\)]*\\)(\\s*\\{[^\\}]*)?\\s*$',
            foldingStopMarker: '^\\s*\\}',
            name: 'Rust',
            scopeName: 'source.rune',
        };

        oop.inherits(RuneHighlightRules, TextHighlightRules);

        exports.RuneHighlightRules = RuneHighlightRules;
    }
);
