# Contributing to the Fuel Rust SDK

Thanks for your interest in contributing to the Fuel Rust SDK!

This document outlines the process for installing dependencies, setting up for development, and conventions for contributing.`

If you run into any difficulties getting started, you can always ask questions on our [Discourse](https://forum.fuel.network/).

## Finding something to work on

You may contribute to the project in many ways, some of which involve coding knowledge and some which do not. A few examples include:

- Reporting bugs
- Adding new features or bug fixes for which there is already an open issue
- Making feature requests

Check out our [Help Wanted](https://github.com/FuelLabs/fuels-rs/labels/help%20wanted) or [Good First Issues](https://github.com/FuelLabs/fuels-rs/labels/good%20first%20issue) to find a suitable task.

If you are planning something big, for example, changes related to multiple components or changes to current behaviors, make sure to [open an issue](https://github.com/FuelLabs/fuels-rs/issues/new) to discuss with us before starting on the implementation.

## Contribution flow

This is a rough outline of what a contributor's workflow looks like:

- Make sure what you want to contribute is already tracked as an issue.
  - We may discuss the problem and solution in the issue.
- Create a Git branch from where you want to base your work. This is usually master.
- Write code, add test cases, and commit your work.
- Run tests and make sure all tests pass.
- Add the breaking label to your PR if the PR contains any breaking changes.
- Push your changes to a branch in your fork of the repository and submit a pull request.
  - Make sure to mention the issue created in step 1 in the commit message.
- Your PR will be reviewed, and some changes may be requested.
  - Your PR must be re-reviewed and approved once you've made changes.
  - Use GitHub's 'update branch' button if the PR becomes outdated.
  - If there are conflicts, you can merge and resolve them locally. Then push to your PR branch. Any changes to the branch will require a re-review.
- Our CI system (Github Actions) automatically tests all authorized pull requests.
- Use GitHub to merge the PR once approved.

Thanks for your contributions!

## Linking issues

Pull requests should be linked to at least one issue in the same repo.

If the pull request resolves the relevant issues, and you want GitHub to close these issues automatically after it merged into the default branch, you can use the syntax (`KEYWORD #ISSUE-NUMBER`) like this:

```sh
close #123
```

If the pull request links an issue but does not close it, you can use the keyword `ref` like this:

```sh
ref #456
```

Multiple issues should use full syntax for each issue and be separated by a comma, like:

```sh
close #123, ref #456
```
