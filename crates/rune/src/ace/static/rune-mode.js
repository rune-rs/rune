ace.define('ace/mode/rune',
    ["require", "exports", "module", "ace/lib/oop", "ace/mode/folding/cstyle", "ace/mode/text", "ace/mode/rune-highlight-rules"],
    function (require, exports, module) {
        "use strict";

        const TextMode = require("ace/mode/text").Mode;
        const FoldMode = require("ace/mode/folding/cstyle").FoldMode;
        const RuneHighlightRules = require("ace/mode/rune-highlight-rules").RuneHighlightRules;
        const oop = require("ace/lib/oop");

        const Mode = function () {
            this.HighlightRules = RuneHighlightRules;
            this.foldingRules = new FoldMode();
            this.$behaviour = this.$defaultBehaviour;
        };

        oop.inherits(Mode, TextMode);

        (function () {
            this.lineCommentStart = "//";
            this.blockComment = { start: "/*", end: "*/", nestable: true };
            this.$quotes = { '"': '"' };
            this.$id = "ace/mode/rune";
        }).call(Mode.prototype);

        exports.Mode = Mode;
    }
);
