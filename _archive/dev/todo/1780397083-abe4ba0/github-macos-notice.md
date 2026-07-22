# Github MacOS migration notice

- [ ] investigate the notice

```txt
 The "macos-latest" label in GitHub Actions will be migrated to the macOS 26 runner image. This migration also includes the “latest” labels for the macOS larger runners:

    macos-latest-large
    macos-latest-xlarge

This change will be rolled out over a period of several weeks beginning June 15, 2026 and will complete by July 15, 2026.

During this period your workflows will gradually switch over to the new image. Once they are migrated, they will not run on macOS 15 in any future runs.

Note that the macOS 26 image may have different tools installed than macOS 15. You can view the tool list for macOS 26 here.

What you need to do:

You do not need to do anything at this time if you want your workflows to migrate to the latest macOS version. If you want to remain on macOS 15 you can follow the below instructions.

    Switch your workflows to the “macos-15” label by changing workflow YAML to use runs-on: macos-15. We support the two latest stable macOS versions plus latest beta, so macOS 15 will be maintained for the next year.

```
