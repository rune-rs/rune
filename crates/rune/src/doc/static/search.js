(function(w) {
    const $doc = w.document;

    let fromPath = null;

    let makePath = (path) => {
        if (!fromPath) {
            return path;
        }

        let newPath = [];
        let i = 0;
        let from = fromPath.split('/');
        let to = path.split('/');

        // strip common prefix.
        while (true) {
            if (from[i] === undefined || to[i] === undefined) {
                break;
            }

            if (from[i] !== to[i]) {
                break;
            }

            i += 1;
        }

        for (let _ of from.slice(i)) {
            newPath.push("..");
        }

        for (let p of to.slice(i)) {
            newPath.push(p);
        }

        return newPath.join('/');
    };

    let getQueryVariable = (variable) => {
        let query = w.location.search.substring(1);
        let vars = query.split('&');

        for (let i = 0; i < vars.length; i++) {
            let pair = vars[i].split('=');

            if (decodeURIComponent(pair[0]) == variable) {
                return decodeURIComponent(pair[1]);
            }
        }

        return null;
    }

    let kindToClass = (kind) => {
        switch (kind) {
        case "function":
            return "fn";
        default:
            return kind;
        }
    };

    let score = (q, item) => {
        let s = 1.0;
        let any = true;
        let itemParts = item.split(":").filter((p) => p !== "").map((p) => p.toLowerCase());

        for (let qp of q.toLowerCase().split(":")) {
            if (qp === "") {
                continue;
            }

            let local = false;
            let lastPart = false;

            for (let part of itemParts) {
                lastPart = false;

                if (part === qp) {
                    local = true;
                    lastPart = true;
                    s *= 2.0;
                } else if (part.startsWith(qp)) {
                    local = true;
                    lastPart = true;
                    s *= 1.5;
                }
            }

            if (lastPart) {
                s *= 2.0;
            }

            if (!local) {
                any = false;
            }
        }

        if (any) {
            return s;
        }

        return null;
    }

    let makeResult = (child, [path, item, kind, doc]) => {
        let linkNode = null;

        if (child.firstChild) {
            linkNode = child.firstChild;
        } else {
            linkNode = $doc.createElement("a");
            child.appendChild(linkNode);
        }

        linkNode.innerHTML = '';

        linkNode.appendChild($doc.createTextNode(kind));
        linkNode.appendChild($doc.createTextNode(" "));

        let parts = item.split("::");

        if (parts.length !== 0) {
            let last = parts[parts.length - 1];

            for (let i = 0; i + 1 < parts.length; i++) {
                if (parts[i] !== "") {
                    linkNode.appendChild($doc.createTextNode(parts[i]));
                    linkNode.appendChild($doc.createTextNode("::"));
                }
            }

            let span = $doc.createElement("span");
            span.className = kindToClass(kind);
            span.appendChild($doc.createTextNode(last));
            linkNode.appendChild(span);
        }

        if (!!doc) {
            linkNode.appendChild($doc.createTextNode(" - "));

            let span = $doc.createElement("span");
            span.className = "inline-docs";
            span.innerHTML = doc;
            linkNode.appendChild(span);
        }

        linkNode.href = makePath(path);
    }

    let removeClass = (els, className) => {
        for (let el of els) {
            if (el.classList.contains(className)) {
                el.classList.remove(className);
            }
        }
    };

    let addClass = (els, className) => {
        for (let el of els) {
            if (!el.classList.contains(className)) {
                el.classList.add(className);
            }
        }
    };

    let baseUrl = () => {
        return w.location.href.split("?")[0].split("#")[0];
    };

    let makeUrl = (q) => {
        if (q !== '') {
            q = encodeURIComponent(q);
            return baseUrl() + `?search=${q}`;
        } else {
            return baseUrl();
        }
    };

    w.addEventListener("load", () => {
        let body = $doc.querySelector("body[data-path]");
    
        if (!!body) {
            fromPath = body.getAttribute("data-path");
        }

        let search = $doc.querySelector("#search");
        let content = $doc.querySelector("#content");

        if (!search || !content || !w.INDEX) {
            return;
        }

        let input = search.querySelector("#search-input");
        let searchResults = search.querySelector("#search-results");
        let searchTitle = search.querySelector("#search-title");

        if (!input || !searchResults || !searchTitle) {
            return;
        }

        let processQuery = (q) => {
            w.history.replaceState('', '', makeUrl(q));

            if (q === '') {
                removeClass([content], "hidden");
                addClass([searchResults, searchTitle], "hidden");
                return;
            }

            addClass([content], "hidden");
            removeClass([searchResults, searchTitle], "hidden");

            let results = [];

            for (let row of w.INDEX) {
                let s = score(q, row[1]);

                if (s !== null) {
                    results.push([s, row]);
                }
            }

            results.sort((a, b) => b[0] - a[0]);

            let i = 0;

            // Make results out of existing child nodes to avoid as much reflow
            // as possible.
            for (let child of searchResults.children) {
                if (i >= results.length) {
                    break;
                }

                let [_, row] = results[i];
                makeResult(child, row);
                i += 1;
            }

            while (i < results.length) {
                const child = $doc.createElement("div");
                child.className = "search-result";
                let [_, row] = results[i];
                makeResult(child, row);
                searchResults.appendChild(child);
                i += 1;
            }

            if (searchResults.children.length !== 0) {
                for (let n = searchResults.children.length - 1; i <= n; n--) {
                    searchResults.removeChild(searchResults.children[n]);
                }
            }
        };

        if (!search.classList.contains("visible")) {
            search.classList.add("visible");
        }

        let q = getQueryVariable("search");

        if (q !== null) {
            processQuery(q);
            input.value = q;
        }

        input.addEventListener("input", (e) => {
            let q = e.target.value;
            processQuery(q);
        });

        let oldAttribute = null;
        
        input.addEventListener("blur", (e) => {
            if (oldAttribute !== null) {
                input.setAttribute("placeholder", oldAttribute);
                oldAttribute = null;
            }
        });

        input.addEventListener("focus", (e) => {
            if (oldAttribute === null) {
                oldAttribute = input.getAttribute("placeholder");
                input.setAttribute("placeholder", "Type your search...");
            }
        });
    });
})(window);
