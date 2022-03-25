# Release

1. From the `main` branch, create a new tag for the new version with the format `v{version_number}` (e.g. `v0.1.0`). This should match the version number in `Cargo.toml`.

```sh
git tag tag v0.1.0
git push origin --tags
```

2. Create a new Github release [here](https://github.com/Shopify/script-runner/releases/new).
3. When the release is created, the Github action defined in `publish.yml` will be run. This will produce the build artifacts for script-runner, except for `arm-macos` (e.g. M1 Macs).
4. Build `script-runner` on an ARM Mac (e.g. M1 Mac)

```sh
gzip -k -f target/release/script-runner && mv target/release/script-runner.gz script-runner-arm-macos-v0.1.0.gz
```

5. Create the shasum file

```sh
shasum -a 256 script-runner-arm-macos-v0.2.0.gz | awk '{ print $1 }' > script-runner-arm-macos-v0.2.0.gz.sha256
```

6. Attach the build and shasum to the release created in step 2.
