// rune-editor code
window.onload = () => {
    // Only permit that snippets run 1_000_000 instructions by default.
    let budget = 1_000_000;
    let outputTrim = 100;
    let outputLineTrim = 80;

    let editors = [];
    
    for (let rune of document.querySelectorAll(".rune")) {
        let updateUrl = rune.getAttribute("rune-update-url") === "true";
        let runOnChange = rune.getAttribute("rune-run-on-change") === "true";
        let runeOptions = parseOptions(rune.getAttribute("rune-options") || "");
        let runeExperimental = rune.getAttribute("rune-experimental") === "true";

        let options = {
            budget,
            outputTrim,
            outputLineTrim,
            updateUrl,
            runOnChange,
            runeOptions,
            runeExperimental
        };

        editors.push(setupEditor(rune, options));
    }

    rune.init().then(() => {
        for (let editor of editors) {
            editor.recompile();
        }
    });
};

function parseOptions(options) {
    let output = [];

    for (let option of options.split(";")) {
        option = option.trim();

        if (!!option) {
            output.push(option);
        }
    }

    return output;
}

function filterPrelude(input) {
    let prelude = [];
    let content = [];

    for (let line of input.split("\n")) {
        if (line.startsWith('#')) {
            prelude.push(line.slice(1));
        } else {
            content.push(line);
        }
    }

    return [prelude.join("\n"), content.join("\n")];
}

function getUrlContent() {
    var query = new URLSearchParams(window.location.search);
    let content = query.get("c");
    
    if (!content) {
        return null;
    }
    
    try {
        return atob(content);
    } catch(e) {
        return null;
    }
}

function updateUrlContent(content) {
    var query = new URLSearchParams(window.location.search);
    query.set("c", btoa(content));
    history.replaceState(null, null, "?" + query.toString());
}

function setupEditor(element, options) {
    let { budget, outputTrim, outputLineTrim, updateUrl, runOnChange, runeOptions, runeExperimental } = options;

    let runeEditor = element.querySelector(".rune-editor");
    let runeOutput = element.querySelector(".rune-output");
    let runButton = element.querySelector(".rune-run");
    let runeControl = element.querySelector(".rune-control");

    if (runOnChange && !!runeControl) {
        runeControl.classList.add("hidden");
    }

    let markers = [];

    let editor = ace.edit(runeEditor, {
        mode: "ace/mode/rune",
        theme: "ace/theme/nord_dark",
        tabSize: 4,
        useSoftTabs: false,
        maxLines: Infinity,
    });

    editor.renderer.setScrollMargin(6, 6, 6, 6);

    let content = editor.getValue();
    let [prelude, newContent] = filterPrelude(content);

    if (!!updateUrl) {
        let content = getUrlContent();

        if (!!content) {
            editor.setValue(content);
        }
    } else {
        editor.setValue(newContent);
    }

    let recompile = async () => {
        if (!rune.module) {
            return;
        }

        if (!!runButton) {
            runButton.disabled = true;
        }

        runeOutput.textContent = "Running...";
        let content = editor.getValue();

        if (!!updateUrl) {
            updateUrlContent(content);
        }

        for (let m of markers) {
            editor.getSession().removeMarker(m);
        }
        
        markers = [];

        if (!!prelude) {
            content = `${content}\n${prelude}`;
        }

        let o = {budget, options: runeOptions, experimental: runeExperimental};
        let result = null;

        try {
            result = await rune.module.compile(content, o);
        } finally {
            if (!!runButton) {
                runButton.disabled = false;
            }
        }

        let text = "";
    
        if (!!result.diagnostics_output) {
            text += result.diagnostics_output + "\n";
        }

        if (!!result.output) {
            let parts = result.output.split("\n").map(part => {
                if (part.length > outputLineTrim) {
                    let trimmed = part.length - outputLineTrim;
                    return part.slice(0, outputLineTrim) + ` ... (${trimmed} trimmed)`;
                } else {
                    return part;
                }
            });
            
            if (parts.length > outputTrim) {
                text += parts.slice(0, outputTrim).join("\n") + "\n";
                text += `${parts.length - outputTrim} more lines trimmed...\n`;
            } else {
                text += parts.join("\n");
            }
        }
        
        if (!result.error) {
            text +=  "== " + result.result;
        }

        runeOutput.textContent = text;
        
        let annotations = [];
        
        for (let d of result.diagnostics) {
            let r = new ace.Range(
                d.start.line, d.start.character,
                d.end.line, d.end.character,
            );
                
            markers.push(editor.getSession().addMarker(r, d.kind, "text"));

            annotations.push({
                row: d.start.line,
                column: d.start.character,
                text: d.message, // Or the Json reply from the parser 
                type: d.kind,
            });
        }

        editor.getSession().clearAnnotations();
        editor.getSession().setAnnotations(annotations);
    };

    if (runOnChange || !runButton) {
        runeOutput.classList.remove("hidden");

        editor.session.on('change', function(delta) {
            recompile();
        });

        return { recompile };
    } else {
        runeOutput.classList.add("hidden");
        runButton.classList.remove("hidden");

        runButton.addEventListener("click", (e) => {
            runeOutput.classList.remove("hidden");
            e.preventDefault();
            recompile();
            return false;
        });

        return { recompile: () => {} };
    }
}
