A small proxy for clojure-lsp to Helix, because Helix doesn't support JAR paths.

The proxy spawns an instance of clojure-lsp and forwards messages to Helix after
rewriting `jar:file://` paths to `file://` paths, extracting the file to the
default caching destination of the OS.

Based on [kotlin-lsp-proxy](https://github.com/Alacho2/kotlin-lsp-proxy).

### Prerequisites

- [clojure-lsp](https://clojure-lsp.io/installation/) must be installed and on your PATH.

### How to

Clone the project, build it with cargo and configure `languages.toml` to point
to the created executable.

    cargo build --release

Then in your Helix `languages.toml`:

    [[language]]
    name = "clojure"
    file-types = ["clj", "cljs", "cljc", "edn"]
    roots = [
      "deps.edn",
      "project.clj",
      "bb.edn",
      "shadow-cljs.edn"
    ]
    language-servers = ["clojure-lsp"]

    [language-server.clojure-lsp]
    command = "/PATH/TO/EXECUTABLE/clojure-lsp-proxy"
