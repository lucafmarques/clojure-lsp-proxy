A small proxy for kotlin-lsp to Helix, because Helix doesn't support JAR paths.

The proxy spawns an instance of kotlin-lsp and forwards messages to Helix after
rewriting the JAR:// path to a file:// path, unzipping the file to the default
caching destination of the OS.

Tested on MacOS, although I haven't done a lot of it, go to definition and
references seems to be working well for me.

### How to.
Clone the project, build it with cargo and configure language.toml to point to
the created executable.
Such as

    [[language]]
    name = "kotlin"
    scope = "source.kotlin"
    injection-regex = "kotlin"
    file-types = ["kt", "kts"]
    shebangs = ["kotlin"]
    roots = [
      "settings.gradle",
      "settings.gradle.kts",
      "build.gradle",
      "build.gradle.kts"
    ]
    comment-token = "//"
    language-servers = ["kotlin-lsp"]
    indent = {tab-width = 2, unit = "  "}


    [language-server.kotlin-lsp]
    command = "/PATH/TO/EXECUTABLE/kotlin-lsp-wrapper"
