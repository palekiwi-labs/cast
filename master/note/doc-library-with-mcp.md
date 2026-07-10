---
status: open
title: Library MCP
---

I frequently clone depencencies that my projects use into a gitignored
`.ref/` subdirectory and luanch research into the source code to understand
the APIs for the features that I want to build. Agents generate `doc`
reports for there depencencies that live in current project repo which means
that if another project also uses this dependency and clone them again,
they cannot benefit from the doc artifacts that have already been
generated in other projects.

The idea that has occured to me is to kee all cloned git repositories in the same
central location (`~/code/`) and create one central AI project that has access to
all of these locations. The agent operating in this project would be able to not only
research each of the projects and access the contents of their `.cue` directories
but also to run comparative research or answer questions that relate to more than
one project or dependency at the same time.

We could call the project "doc library" or something like that and expose its agent
as a service to other projects via an MCP. Imagine that you as an agent want to ask
a question about a particular Rust crate or an npm package, so you call this MCP
tool. On the server side, the tool will launch an agent in our "doc library" which
will handle your question. The agent may either be restricted to only those repos
that have already been cloned or it may be allowd to pull in new repositories on
demand in order to answer the question. The "doc libarary" agent would still use
the "cue" framework in order to persist the results of its research as doc
artifacts.

The MCP tool should probably allow calling agents to specify which package they
want to enquire about as well as the version of the package.
