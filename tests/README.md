# Integration tests

This directory is for black-box tests of the compiled `agent-harness` binary. Tests should exercise the application through its command-line arguments, environment, standard input, standard output, standard error, and exit status rather than importing private modules from `src`.

The first suite should be added with the single-prompt CLI. It should run the binary against a local fake OpenAI endpoint and cover these observable scenarios:

- a prompt produces and prints an assistant response;
- a tool call is executed and followed by a final response;
- provider and response-format failures produce clear errors and unsuccessful exit statuses;
- tool-call IDs and ordering survive the complete model/tool loop;
- provider-turn and tool-call limits stop repeated calls.

Keep shared test infrastructure, such as the fake HTTP provider, under `tests/support/`. Avoid importing source files with `#[path]`; doing so couples the suite to implementation structure rather than application behaviour.
