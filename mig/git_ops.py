import os
from git import Repo

def push_code(filepaths: list[str], commit_message: str):
    """
    Adds files to the git repository, commits them, and pushes to origin.
    """
    repo_path = os.getcwd()
    try:
        repo = Repo(repo_path)
    except Exception as e:
        raise RuntimeError(f"Current directory {repo_path} is not a git repository.") from e
    
    if repo.is_dirty(untracked_files=True):
        repo.index.add(filepaths)
        repo.index.commit(commit_message)
        
        origin = repo.remote(name='origin')
        origin.push()
    else:
        print("No changes to commit.")
