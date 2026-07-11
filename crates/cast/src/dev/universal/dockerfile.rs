use crate::dev::agent::Agent;

/// Shared top section of every universal Dockerfile: base image, build args
/// required by arch detection, the union apt install, and the Nix environment.
const PREAMBLE: &str = "\
FROM debian:trixie-slim

ARG TARGETARCH

# Install essential tools for development (union of all agents' deps)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    ssh \
    && rm -rf /var/lib/apt/lists/*

# Configure Nix system-wide to enable flakes
# (Nix store will be mounted from nix-daemon volume)
RUN mkdir -p /etc/nix && \
    echo \"experimental-features = nix-command flakes\" > /etc/nix/nix.conf

# Add Nix to PATH (will be available if /nix is mounted)
ENV PATH=\"/nix/var/nix/profiles/default/bin:${PATH}\"

# Limit GC marker threads for Nix commands running in the container
ENV GC_NPROCS=1

# Ensure Nix client communicates via daemon socket (store is mounted read-only)
ENV NIX_REMOTE=daemon";

/// Shared bottom section: git safe-directory, PATH, and user + directory
/// creation. The mkdir covers the union of all known agent config dirs.
const POSTAMBLE: &str = "\
USER root

# Allow git operations on bind-mounted workspaces owned by a different UID.
RUN git config --system safe.directory \"*\"

# Ensure /usr/local/bin is in PATH (defensive for custom base images)
ENV PATH=\"/usr/local/bin:${PATH}\"

ARG USERNAME=user
ARG UID=1000
ARG GID=1000
ARG EXTRA_DIRS=\"\"

# Create user and union of agent directories (idempotent - safe even if user/dirs exist)
RUN set -e; \
    if getent group ${GID} >/dev/null 2>&1; then \
        GROUP_NAME=$(getent group ${GID} | cut -d: -f1); \
    else \
        groupadd -g ${GID} --non-unique ${USERNAME} 2>/dev/null || true; \
        GROUP_NAME=${USERNAME}; \
    fi && \
    if ! getent passwd ${USERNAME} >/dev/null 2>&1; then \
        useradd -u ${UID} -g ${GID} -m -d /home/${USERNAME} --non-unique -s /bin/bash ${USERNAME} 2>/dev/null || true; \
    fi && \
    mkdir -p /home/${USERNAME} && \
    mkdir -p /workspace \
             /home/${USERNAME}/.cache \
             /home/${USERNAME}/.claude \
             /home/${USERNAME}/.pi \
             /home/${USERNAME}/.config \
             /home/${USERNAME}/.local \
             ${EXTRA_DIRS} && \
    chown -R ${UID}:${GID} /workspace \
                           /home/${USERNAME}/.cache \
                           /home/${USERNAME}/.claude \
                           /home/${USERNAME}/.pi \
                           /home/${USERNAME}/.config \
                           /home/${USERNAME}/.local \
                           /home/${USERNAME} \
                           ${EXTRA_DIRS} 2>/dev/null || true

USER ${USERNAME}
WORKDIR /workspace

CMD [\"bash\"]";

/// Assemble a universal Dockerfile from the shared preamble/postamble and the
/// per-agent installation fragments contributed by each included agent.
///
/// Fragments are emitted in sorted order by agent name so the assembled file is
/// deterministic for a given set of agents.
pub fn assemble(agents: &[&dyn Agent]) -> String {
    let mut sorted: Vec<&dyn Agent> = agents.to_vec();
    sorted.sort_by_key(|a| a.name());

    let mut sections: Vec<String> = vec![PREAMBLE.to_string()];
    for agent in &sorted {
        let snippet = agent.dockerfile_snippet().trim();
        if !snippet.is_empty() {
            sections.push(snippet.to_string());
        }
    }
    sections.push(POSTAMBLE.to_string());
    sections.join("\n\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::claudecode::ClaudeCode;
    use crate::dev::opencode::OpenCode;
    use crate::dev::pi::Pi;

    #[test]
    fn assemble_opencode_contains_fragment_and_structure() {
        let oc = OpenCode;
        let agents: &[&dyn Agent] = &[&oc];
        let df = assemble(agents);
        assert!(
            df.contains("anomalyco/opencode"),
            "missing opencode install url"
        );
        assert!(df.contains("ARG TARGETARCH"), "missing TARGETARCH");
        assert!(
            df.contains("git config --system safe.directory"),
            "missing safe.directory"
        );
    }

    #[test]
    fn assemble_claudecode_copy_node_before_npm_install() {
        let cc = ClaudeCode;
        let agents: &[&dyn Agent] = &[&cc];
        let df = assemble(agents);
        let copy_pos = df
            .find("COPY --from=node")
            .expect("expected COPY --from=node in claudecode assemble");
        let npm_pos = df
            .find("npm install")
            .expect("expected npm install in claudecode assemble");
        assert!(
            copy_pos < npm_pos,
            "COPY --from=node must come before npm install"
        );
    }

    #[test]
    fn assemble_opencode_pi_excludes_claudecode() {
        let oc = OpenCode;
        let pi = Pi;
        let agents: &[&dyn Agent] = &[&oc, &pi];
        let df = assemble(agents);
        assert!(
            df.contains("anomalyco/opencode"),
            "missing opencode fragment"
        );
        assert!(df.contains("badlogic/pi-mono"), "missing pi fragment");
        assert!(
            !df.contains("npm install"),
            "claudecode npm install must be absent"
        );
        assert!(
            !df.contains("COPY --from=node"),
            "claudecode COPY --from=node must be absent"
        );
    }

    #[test]
    fn assemble_all_three_contains_every_fragment() {
        let oc = OpenCode;
        let cc = ClaudeCode;
        let pi = Pi;
        let agents: &[&dyn Agent] = &[&oc, &cc, &pi];
        let df = assemble(agents);
        assert!(df.contains("anomalyco/opencode"), "missing opencode");
        assert!(df.contains("npm install"), "missing claudecode");
        assert!(df.contains("badlogic/pi-mono"), "missing pi");
        assert!(df.contains("ARG TARGETARCH"), "missing TARGETARCH");
        assert!(
            df.contains("git config --system safe.directory"),
            "missing safe.directory"
        );
    }

    #[test]
    fn assemble_is_independent_of_input_order() {
        let oc = OpenCode;
        let cc = ClaudeCode;
        let pi = Pi;
        let forward: &[&dyn Agent] = &[&oc, &cc, &pi];
        let reverse: &[&dyn Agent] = &[&pi, &cc, &oc];
        // Fragments must appear in sorted name order: claudecode, opencode, pi.
        let df = assemble(forward);
        let cc_pos = df.find("CLAUDECODE_VERSION").unwrap();
        let oc_pos = df.find("OPENCODE_VERSION").unwrap();
        let pi_pos = df.find("PI_VERSION").unwrap();
        assert!(cc_pos < oc_pos, "claudecode must precede opencode");
        assert!(oc_pos < pi_pos, "opencode must precede pi");
        // Reordering the input must not change the output.
        assert_eq!(df, assemble(reverse));
    }
}
