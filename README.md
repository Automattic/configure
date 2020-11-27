# Configure

A tool for storing encrypted secrets in your repository, and decrypting them in CI. It allows you to store your configuration files in your source repository, even if that repository is public.

## How to use it

The configure tool has two main jobs: copy plain-text secrets files from your secrets repository into the project as encrypted blobs, and decrypting those blogs back into the plain-text files on developer and build machines.

### Setup

`configure setup` will walk you through the process of setting up your project.