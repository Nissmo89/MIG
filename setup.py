from setuptools import setup, find_packages

setup(
    name="mig-bot",
    version="0.1.0",
    description="MIG (Make It Green) - An auto-contributing bot with personality and memory",
    author="MIG Developer",
    packages=find_packages(),
    install_requires=[
        "langchain>=0.1.0",
        "langchain-groq>=0.0.1",
        "langchain-openai>=0.0.8",
        "langchain-google-genai>=1.0.0",
        "langgraph>=0.0.26",
        "langgraph-checkpoint-sqlite>=1.0.0",
        "gitpython>=3.1.41",
        "python-dotenv>=1.0.1",
        "click>=8.1.7",
    ],
    entry_points={
        "console_scripts": [
            "mig=mig.cli:main",
        ],
    },
)
