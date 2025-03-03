# AIGitCommit

![screenshots](./assets/screenshots.png)

A simple tool to help you write better Git commit messages using AI.

## Features

- Generates meaningful commit messages based on your code changes
- Easy-to-use command-line interface
- By using the libgit2 library, there is no need to call an external command for security reasons
- The system supports multiple AI models that are compatible with the OpenAI API
- Socks5 and HTTP proxy supported
- Supports integration with Git workflow (todo)

## How It Works

AIGitCommit looks at your Git staged changes and uses AI to make commit lines that are clear and helpful. 

It looks at the diff result and uses machine learning to figure out what your changes were meant to do and why you made them. It then generates a commit message that is clear and helpful.

## Install

AIGitCommit is still in the early stages of development, I suggest you to install it using the git URL using the commands below:

```
cargo install --git https://github.com/mingcheng/aigitcommit.git
```

This command will auto-download the latest version of the project and install it to your cargo bin directory.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

`- eof -`