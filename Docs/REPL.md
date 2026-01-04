A REPL (Read-Eval-Print Loop) is an interactive loop that forms the core of a shell. It follows a repeating cycle:

    Read: Display a prompt and wait for user input
    Eval: Parse and execute the command
    Print: Display the output or error message
    Loop: Return to step 1 and wait for the next command

This cycle continues indefinitely until the shell process is terminated.

Your shell should follow this same cycle:

    Display the prompt $, then wait for a line of input.
    Print <command_name>: command not found for any command the user enters, like with the previous stages.
    Return to step 1.

For example, if the user types hello, your shell should print hello: command not found, then display the prompt ($) again.