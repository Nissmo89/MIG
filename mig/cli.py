import click
import os
from dotenv import load_dotenv
from mig.bot import run_bot

@click.group()
def main():
    """MIG (Make It Green) - Auto Contributing Bot"""
    load_dotenv()

@main.command()
def init():
    """Initialize a git repository if not already done, and create a Context.md."""
    if not os.path.exists(".git"):
        os.system("git init")
        click.echo("Initialized empty Git repository.")
    else:
        click.echo("Git repository already exists.")
        
    if not os.path.exists("Context.md"):
        with open("Context.md", "w") as f:
            f.write("# MIG Personality Context\n\nYou are an auto-contributing bot. Generate small Python scripts.\n")
        click.echo("Created default Context.md.")
    else:
        click.echo("Context.md already exists.")

@main.command()
def run():
    """Run the bot to make a contribution."""
    click.echo("Running MIG...")
    try:
        run_bot()
        click.echo("MIG finished successfully!")
    except Exception as e:
        click.echo(f"Error running MIG: {e}", err=True)

@main.command()
def install_cron():
    """Install a cron job to run MIG daily (system startup or specific time)."""
    click.echo("To run MIG on startup, you can add the following to your crontab (crontab -e):")
    click.echo("@reboot cd /path/to/repo && mig run")
    click.echo("Or to run daily at 9 AM:")
    click.echo("0 9 * * * cd /path/to/repo && mig run")

if __name__ == "__main__":
    main()
