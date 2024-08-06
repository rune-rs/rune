ace.define('ace/autocomplete/rune',
    ["require", "exports", "module"],
    function (require, exports, module) {
        exports.Completer = {
            getCompletions: (editor, session, pos, prefix, callback) => {
                if (prefix.length === 0) {
                    callback(null, []);
                    return;
                }

                var token = session.getTokenAt(pos.row, pos.column - 1).value;

                if (token.includes(".")) {
                    callback(null, instance);
                } else {
                    callback(null, fixed);
                }
            },
        };
    }
);
