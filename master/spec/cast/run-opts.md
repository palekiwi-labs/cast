# Cast Run Options

> TODO: Help me fill out a specification
> for the following <idea>

Currently the only way to run an agent is with the `cast run <agent>` command which
takes only one argument and then runs it in the interactive mode with tty attached.

Imagine a situation though, that we want to run an agent headlessly with a prompt
in a way that returns or streams a json response. Agent harnesses support that,
for example this is what `opencode` agent supports: `/home/pl/code/palekiwi-labs/cast/.cue/master/ref/1781804980-2ce36c4/opencode-run--help.txt`

There isn't currently a way to do this.

We should also consider how to handle these situations:
- a dev container is already running (user has run `cast run opencode`)
- a dev container is not running

We need to consider whether we want to wrap a `docker run` command
or a `docker exec` command. I lean towards a `docker run` which spins up a new
container because that makes our process independent of the lifecycle of any
other container, e.g. a user terminates the main container which also kills
our process.

So perhaps a solution would be to support something like:
`cast run <flags>` where the flags can be defined to determine:

- should the container be interactive with a TTY attached?
- what should be the name of the container so that it does not conflict with the defaultly named one
- what about port publishing? it would conflict and we probaly do not want port publishing for headless containers anyway
