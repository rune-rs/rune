// rune-editor code
window.onload = () => {
    // Only permit that snippets run 1_000_000 instructions by default.
    let budget = 1_000_000;
    let outputTrim = 100;
    let outputLineTrim = 80;

    let editors = [];
    
    for (let rune of document.querySelectorAll(".rune")) {
        let updateUrl = rune.getAttribute("rune-update-url") === "true";
        editors.push(setupEditor(rune, {budget, outputTrim, outputLineTrim, updateUrl}));
    }

    rune.init().then(() => {
        for (let editor of editors) {
            editor.recompile();
        }
    });
};

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
    let { budget, outputTrim, outputLineTrim, updateUrl } = options;

    let runeEditor = element.querySelector(".rune-editor");
    let runeTitle = element.querySelector(".rune-title");
    let runeOutput = element.querySelector(".rune-output");

    let markers = [];

    let editor = ace.edit(runeEditor, {
        mode: "ace/mode/rust",
        theme: "ace/theme/nord_dark",
        tabSize: 2,
        useSoftTabs: false,
        maxLines: Infinity,
    });

    editor.renderer.setPadding(8)

    if (!!updateUrl) {
        let content = getUrlContent();

        if (!!content) {
            editor.setValue(content);
        }
    }

    let recompile = () => {
        if (!rune.module) {
            return;
        }

        let content = editor.getValue();
        
        if (!!updateUrl) {
            updateUrlContent(content);
        }
        
        for (let m of markers) {
            editor.getSession().removeMarker(m);
        }
        
        markers = [];
        
        let result = rune.module.compile(content, budget);
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
        
        if (!!result.error) {
            if (!!runeTitle) {
                runeTitle.textContent = "Error when running snippet";
            }
        } else {
            text +=  "== " + result.result;

            if (!!runeTitle) {
                runeTitle.textContent = "Output";
            }
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
    
    editor.session.on('change', function(delta) {
        recompile();
    });

    return { recompile };
}