# Configure

A tool for storing encrypted secrets in your repository, and decrypting them in CI. It allows you to store your configuration files in your source repository, even if that repository is public.

## How to use it

The configure tool has two main jobs: copy plain-text secrets files from your secrets repository into the project as encrypted blobs, and decrypting those blogs back into the plain-text files on developer and build machines.

Once you add the plugin to your project, there will be 2 Gradle tasks available to you:
1. `updateConfiguration`: Update the encrypted configuration files from the secrets repository -> You should use this task when you have made a change to the secrets repository (updates project's `.configure` file)
2. `applyConfiguration`: Apply the encrypted configuration -> You should use this task to copy & decrypt the secrets (applies project's `.configure` file)

### Setup

`configure setup` will walk you through the process of setting up your project.
