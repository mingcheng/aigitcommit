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

## Configuration

Initially, you must configure your `OPENAI_*` environment variables to request prompts from an OpenAI-compatible API service. Set them as follows in your shell configuration file:

- `OPENAI_API_TOKEN`: Your individual OpenAI token
- `OPENAI_API_BASE`: Your specified openAI request base
- `OPENAI_MODEL_NAME`: Give the model name you wish to request

If your network requirements a proxy to access the API service, you must specify the proxy address using the `OPENAI_API_PROXY` environment variable. 

For instance, `http://127.0.0.1:1080` is suitable for an HTTP proxy, while `socks://127.0.0.1:1086` is an appropriate choice for a Socks5 proxy.

## Usage

The way to use AIGitComment is really simple. For example, you can run `aigitcoment` in the current directory after staging the file to have git commits generated automatically before git commit. Additionally, you may provide the git directory using `aigitcommit <dir>`.

If you would like more usage settings, just use `aigitcommit --help` to get more details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

`- eof -`