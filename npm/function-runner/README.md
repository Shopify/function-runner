# function-runner npm package

This is the npm package for function-runner. The package contains a small Node
script that downloads the appropriate function-runner binary on demand and
invokes it with the parameters given.

## Usage

```
# Install function-runner globally
$ npm install -g function-runner

# Directly invoke it via npm
$ npx function-runner
```

## Updating function-runner

The npm package won't download function-runner again once its downloaded. If a
new version of the function-runner npm package has been published, you can use
the following invocation to update to the latest release:

```
REFRESH_FR=1 npx function-runner
```

## Building from source

If there are no binaries available for your platform or the available binaries
don't work for you for some reason, the npm package can also build function-
runner from  source.

```
BUILD_FR=1 npx function-runner
```

Please note that for this to work you must have Rust installed.


