# Tui-pages

[![Book](https://img.shields.io/badge/mdBook-online-brightgreen)](https://tui-pages.farmeris.sk)
[![Docs](https://img.shields.io/badge/docs-rs-orange)](https://docs.rs/tui-pages)


> [!IMPORTANT]
> Read this readme, its handwritten. mdBook is well written but AI assistance was used

> **Origin:** Originally developed for TUI accounting system. This crate was extracted from it and further generalized.

## Docs:
```
git submodule update --init --recursive
mdbook serve mdbook --open
```
## Motivation
You want to create complex TUI app? Me too. I did it and was stuck with god object mutating shared reference everywhere(I know, skill issues). So I had to rewrite the whole architecture. This crate is that architecture. Asked AI to generalize it for you ;)

Its a full framework. You get all the functionality out of the box. 

## Why you **should** use this crate

It fixes TUI problems. But how?
Ill simplify. Imagine 0..n element IDs. Now Im at ID 1 and want to move next. So Ill move to 2. Thats what this library is. There is a functionality programmed on top of such "list". And what do you do? You just write "this page will have 3 buttons, assign id 0, 1, 2 to those".
What are the implications:
1. Fully preprogrammed functionality. You assign elements, crate handles vim or vscode like movements.  
2. You get focus manager. Its preprogrammed on the list, so it will always work.
3. Most important. Scalability across pages. I developed complex system, with many pages. And I need most of the functionality to work the same across them(mainly movement, texts, buttons...). If you have multiple pages, I cant imagine writing the app different way.

Restrictions:
There are none, you can extend this however you want.

## Actual docs
### How it worked before the generalization:
<img src="docs/full_system_legacy.mermaid.svg" width="400">

#### Simply:

User pressed keybinding(`"ctrl+s"` - handled by KeyEvent), or any keyboard letter. That is flushed into InputPipeline.

InputPipeline - maps typesafe `Command::Save` to string from KeyEvent or keyboard.

InputOrchestrator - decides where the user request should go. E.g. we press `"j"` for `Movement::Down`. This request is to be processed by FocusManager. So we just go there. 

FocusManager `IMPORTANT` - this handles focus. Of the whole app. Like it holds everything. So if library wants focus, its a problem, there is system inside of it for that. But if you want to focus element, you simply tell focus manager. You are not doing it on your own, be dumb, let this do the work for you.

CommandPipeline - I forgot, who cares

ActionDecider - Who does what

Executor - function lives in the login page for login. We should login. So executor does call this function from the login page. Its a simple function call.

## Examples
There are 4, located at examples dir, go into whichever you want to run and hit cargo run.

## Architecture

(Dont even read this, its done by Opus, its for the LLMs)
See [`docs/architecture/architecture.md`](docs/architecture/architecture.md)
for the full design, flow diagrams, and the primitive layer.
