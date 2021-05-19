# Configure Ruby Example Project

The example project allows you to run commands using `bundle exec`.

## Init

Run `bundle exec configure_init` to interactively initialize a repository.

## Apply

Run `bundle exec configure_apply` to decrypt `.enc` files in the `.configure-files` directory.

## Update

Run `bundle exec configure_update` to copy files from the secrets repo into the `.configure-files` directory, encrypted with the project's secret key.
