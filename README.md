_droplet_ is a tiny Rust application whose sole purpose is to receive tarballs as HTTP `PATCH` requests and unpack them into a specified directory.
This application is intended to be used as a sidecar via which your pipelines can deploy files for a static site.
This can be done by having it share a volume with a server such as [static-web-server](https://github.com/joseluisq/static-web-server).

droplet includes no authorization mechanism, so make sure it's only reachable from trusted systems.

## Configuration

droplet is configured through these environment variables:

| Variable             | default        | Description                                            |
| -------------------- | -------------- | ------------------------------------------------------ |
| `DROPLET_ADDRESS`    | `0.0.0.0:3000` | The socket adress on which to listen for http requests |
| `DROPLET_TARGET_DIR` | `/target`      | The directory into which tarballs are unpacked         |

## Interface
Say droplet is running on `localhost:3000` then sending a `PATCH` with an uncompressed tarball body to any path on that adress will unpack the tar into the target directory.
E.g. with httpie:
```bash
http patch localhost:3000/ < archive.tar
```
