import os
from langchain_groq import ChatGroq
from langchain_openai import ChatOpenAI
try:
    from langchain_google_genai import ChatGoogleGenerativeAI
except ImportError:
    pass

def get_llm():
    """
    Initialize the LLM based on available environment variables.
    Checks for GEMINI_API_KEY, GROQ_API_KEY, and OPENROUTER_API_KEY.
    """
    if os.getenv("GEMINI_API_KEY") or os.getenv("GOOGLE_API_KEY"):
        return ChatGoogleGenerativeAI(
            temperature=0.7,
            model=os.getenv("GEMINI_MODEL", "gemini-2.5-flash-lite")
        )
    elif os.getenv("GROQ_API_KEY"):
        return ChatGroq(
            temperature=0.7,
            model_name=os.getenv("GROQ_MODEL", "llama-3.1-8b-instant")
        )
    elif os.getenv("OPENROUTER_API_KEY"):
        return ChatOpenAI(
            temperature=0.7,
            api_key=os.getenv("OPENROUTER_API_KEY"),
            base_url="https://openrouter.ai/api/v1",
            model=os.getenv("OPENROUTER_MODEL", "openrouter/free")
        )
    else:
        raise ValueError("GEMINI_API_KEY, GROQ_API_KEY, or OPENROUTER_API_KEY must be set in the environment.")
