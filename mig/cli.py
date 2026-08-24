import click
import os
import json
from dotenv import load_dotenv
from mig.bot import run_bot

def track_project(path):
    stats_dir = os.path.expanduser("~/.mig")
    os.makedirs(stats_dir, exist_ok=True)
    proj_file = os.path.join(stats_dir, "projects.json")
    
    projects = []
    if os.path.exists(proj_file):
        try:
            with open(proj_file, "r") as f:
                projects = json.load(f)
        except Exception:
            pass
            
    if path not in projects:
        projects.append(path)
        with open(proj_file, "w") as f:
            json.dump(projects, f)

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
        
    # Track project
    track_project(os.getcwd())
        
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
    track_project(os.getcwd())
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

@main.command()
def dashboard():
    """Launch the interactive MIG dashboard."""
    # Locate the mig-dashboard rust project
    dash_path = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "mig-dashboard"))
    if os.path.exists(dash_path):
        os.system(f"cd {dash_path} && cargo run -q")
    else:
        click.echo("mig-dashboard Rust project not found.")

if __name__ == "__main__":
    main()
