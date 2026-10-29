# ChatGPT

A desktop chat client built from the `gpui_kit::component::ai` elements
(`PromptInput`, `ModelSelector`, `Reasoning`, `Tool`, `ContextUsage`,
`Suggestion`) over `MessageScroller` and `Message`.

```bash
cargo run -p example-chatgpt
```

It includes:

- a sidebar with chat search, projects, and chats grouped by date;
- projects with their own instructions, added to every chat inside them;
- image attachments (paperclip or drag and drop) and image generation, with
  partial images refining in place and **Save image…**;
- reasoning summaries, web searches, cited sources, copy and regenerate;
- history, projects and images saved under the platform config directory
  (`gpui-kit-chatgpt/`), so conversations reopen on launch.

## Signing in

- **ChatGPT account:** choose **Sign in with ChatGPT…**. The app runs the
  OAuth authorization-code flow with PKCE against `auth.openai.com`, receives
  the redirect on `http://localhost:1455/auth/callback` and stores the tokens
  in the platform config directory (`gpui-kit-chatgpt/auth.json`, mode 0600).
  Expiring access tokens are refreshed before each request. An existing
  `codex login` (`~/.codex/auth.json`) is reused without signing in again.
- **API key:** set `OPENAI_API_KEY` to call `https://api.openai.com/v1/responses`
  instead.

## How requests are made

Both modes send the same stateless Responses API request (`stream: true`,
`store: false`, the whole conversation as `input`, `reasoning.summary: "auto"`,
the `web_search` tool and the `image_generation` tool with partial images) and read its server-sent events:
`response.output_text.delta`, `response.reasoning_summary_text.delta`,
`web_search_call` and `image_generation_call` items, partial images,
`url_citation` annotations and `response.completed` usage. Attached images are
sent as `input_image` data URLs.

A ChatGPT sign-in goes through `https://chatgpt.com/backend-api/codex/responses`
with the `chatgpt-account-id` header, and the model menu is read from
`/backend-api/codex/models`, so it shows the models and reasoning efforts the
account may use.

## Caveats

The ChatGPT sign-in uses the Codex CLI's public OAuth client and its
undocumented backend. It is not an official OpenAI API, it may change without
notice, and its usage counts against the account's Codex limits.

| Variable | Purpose |
| --- | --- |
| `OPENAI_API_KEY` | Use an API key when no ChatGPT sign-in exists |
| `CHATGPT_MODELS` | Comma-separated model slugs replacing the model menu |
| `CHATGPT_INSTRUCTIONS` | Replace the system instructions |
