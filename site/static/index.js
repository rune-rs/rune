// rune-editor code
window.addEventListener("load", () => {
    // Only permit that snippets run 1_000_000 instructions by default.
    let budget = 1_000_000;
    let colTrim = 100;
    let lineTrim = 80;

    let editors = [];
    
    for (let rune of document.querySelectorAll(".rune")) {
        let updateUrl = rune.getAttribute("rune-update-url") === "true";
        let runOnChange = rune.getAttribute("rune-run-on-change") === "true";
        let options = parseOptions(rune.getAttribute("rune-options") || "");
        let experimental = rune.getAttribute("rune-experimental") === "true";
        let config = parseConfig(rune.getAttribute("rune-config"));

        let opts = {
            budget,
            colTrim,
            lineTrim,
            updateUrl,
            runOnChange,
            options,
            experimental,
            config,
        };

        editors.push(setupEditor(rune, opts));
    }

    rune.init().then(() => {
        for (let editor of editors) {
            editor.recompile();
        }
    });
});

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

function parseConfig(config) {
    if (!config) {
        return {};
    }

    return JSON.parse(config);
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
        return atou(content);
    } catch(e) {
        return null;
    }
}

function updateUrlContent(content) {
    var query = new URLSearchParams(window.location.search);

    try {
        query.set("c", utoa(content));
    } catch (e) {
        return;
    }

    history.replaceState(null, null, "?" + query.toString());
}

function setupEditor(element, opts) {
    let runeEditor = element.querySelector(".rune-editor");

    let primaryOutput = element.querySelector(".rune-output.primary");
    let diagnosticsOutput = element.querySelector(".rune-output.diagnostics");
    let instructionsOutput = element.querySelector(".rune-output.instructions");

    let runButton = element.querySelector(".rune-run");
    let instructionsCheckbox = element.querySelector(".rune-checkbox.instructions");
    let runOnChangeCheckbox = element.querySelector(".rune-checkbox.run-on-change");

    primaryOutput.classList.add("hidden");
    diagnosticsOutput.classList.add("hidden");
    instructionsOutput.classList.add("hidden");

    if (!!runOnChangeCheckbox) {
        runOnChangeCheckbox.checked = opts.runOnChange;
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

    if (!!opts.updateUrl) {
        let content = getUrlContent();

        if (!!content) {
            editor.setValue(content);
        }
    } else {
        editor.setValue(newContent);
    }

    let recompile = async () => {
        if (!rune.module) {
            console.warn("Rune module not available");
            return;
        }

        if (!!runButton) {
            runButton.disabled = true;
        }

        primaryOutput.textContent = "Running...";
        primaryOutput.classList.remove("hidden");

        let content = editor.getValue();

        if (!!opts.updateUrl) {
            updateUrlContent(content);
        }

        for (let m of markers) {
            editor.getSession().removeMarker(m);
        }
        
        markers = [];

        if (!!prelude) {
            content = `${content}\n${prelude}`;
        }

        let o = {
            budget: opts.budget,
            options: opts.options,
            experimental: opts.experimental,
            instructions: !!(instructionsCheckbox && instructionsCheckbox.checked),
            ...opts.config,
        };

        let result = null;

        try {
            result = await rune.module.compile(content, o);
        } finally {
            if (!!runButton) {
                runButton.disabled = false;
            }
        }

        if (!!result.diagnostics_output) {
            diagnosticsOutput.textContent = result.diagnostics_output;
            diagnosticsOutput.classList.remove("hidden");
        } else {
            diagnosticsOutput.textContent = "";
            diagnosticsOutput.classList.add("hidden");
        }

        let text = [];

        if (!!result.output) {
            let content = [];

            let parts = result.output.split("\n").map(part => {
                if (part.length > opts.lineTrim) {
                    let trimmed = part.length - opts.lineTrim;
                    return part.slice(0, opts.lineTrim) + ` ... (${trimmed} trimmed)`;
                } else {
                    return part;
                }
            });

            if (parts.length > opts.colTrim) {
                content.push(...parts.slice(0, opts.colTrim));
                content.push(`${parts.length - opts.colTrim} more lines trimmed...`);
            } else {
                content.push(...parts);
            }

            text.push({ title: "Output", content });
        }

        if (!!result.error) {
            text.push({ title: "Result", content: [result.error] });
        }

        if (!!result.result) {
            text.push({ title: "Result", content: [result.result] });
        }

        if (text.length > 0) {
            let el = document.createDocumentFragment();

            for (let section of text) {
                let title = document.createElement("h4");
                title.textContent = section.title;

                el.appendChild(title);

                for (let line of section.content) {
                    let text = document.createElement("p");
                    text.textContent = line;
                    el.appendChild(text);
                }
            }

            primaryOutput.innerHTML = "";
            primaryOutput.appendChild(el);
            primaryOutput.classList.remove("hidden");
        } else {
            primaryOutput.innerHTML = "";
            primaryOutput.classList.add("hidden");
        }

        if (!!result.instructions) {
            instructionsOutput.textContent = `# instruction\n${result.instructions}`;
            instructionsOutput.classList.remove("hidden");
        } else {
            instructionsOutput.textContent = "";
            instructionsOutput.classList.add("hidden");   
        }

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

    runButton.addEventListener("click", (e) => {
        e.preventDefault();
        recompile();
        return false;
    });

    editor.session.on('change', function(delta) {
        if (runOnChangeCheckbox.checked) {
            recompile();
        }
    });

    if (runOnChangeCheckbox.checked) {
        return { recompile };
    }

    return { recompile: () => {} };
}


/**
 * ASCII to Unicode (decode Base64 to original data)
 */
function atou(b64) {
    return decodeURIComponent(escape(atob(b64)));
}

/**
 * Unicode to ASCII (encode data to Base64)
 */
function utoa(data) {
    return btoa(unescape(encodeURIComponent(data)));
}
