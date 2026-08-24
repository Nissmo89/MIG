import os
import sqlite3
import json
from langchain_core.messages import SystemMessage, HumanMessage, AIMessage
from langgraph.graph import StateGraph, END
from langgraph.checkpoint.sqlite import SqliteSaver
from typing import TypedDict, Annotated, List, Optional
import operator
from datetime import datetime
import time

from mig.llm import get_llm
from mig.git_ops import push_code

from langgraph.graph.message import add_messages

# Define State for LangGraph
class AgentState(TypedDict):
    messages: Annotated[List[SystemMessage | HumanMessage | AIMessage], add_messages]
    context: str
    current_level: int
    past_problems: List[str]
    code_to_commit: Optional[str]
    commit_message: Optional[str]
    filename: Optional[str]

def load_context():
    context_path = os.path.join(os.getcwd(), "Context.md")
    if os.path.exists(context_path):
        with open(context_path, "r") as f:
            return f.read()
    return "You are an auto-contributing bot."

def read_memory(state: AgentState):
    # This node could initialize state if needed, but SqliteSaver handles message history.
    # We can also parse the history to find past problems and current level.
    # For simplicity, we just pass the context along.
    context = load_context()
    
    # We can inject a human message asking for today's contribution.
    msg = HumanMessage(content="It's a new day! Please generate a new, clever DSA code snippet or micro-algorithm. Pick ANY language (Rust, C, C++, Python, Java, Go). Write purely hand-written style code (no AI boilerplate, no defensive null checks). Return the exact relative path using a language folder (e.g. `rust/trick.rs`), the commit message, and the code block.")
    
    return {"context": context, "messages": [msg]}

def generate_contribution(state: AgentState):
    llm = get_llm()
    system_prompt = state.get("context", "You are an auto-contributing bot.")
    system_msg = SystemMessage(content=system_prompt + "\n\nOutput ONLY a JSON block (and nothing else) with keys: 'filename', 'commit_message', 'code'. The 'code' should be the raw code string.")
    
    messages = [system_msg] + state["messages"]
    
    start_time = time.time()
    response = llm.invoke(messages)
    
    try:
        # Parse JSON from response
        # Sometimes LLMs wrap in ```json ... ```
        content = response.content.strip()
        if content.startswith("```json"):
            content = content[7:]
        if content.startswith("```"):
            content = content[3:]
        if content.endswith("```"):
            content = content[:-3]
        
        data = json.loads(content.strip())
        
        # --- Update Global Stats ---
        duration = time.time() - start_time
        tokens = 0
        if hasattr(response, 'usage_metadata') and response.usage_metadata:
            tokens = response.usage_metadata.get('total_tokens', 0)
        if tokens == 0:
            # Fallback estimation
            tokens = (len(str(messages)) + len(content)) // 4
            
        stats_dir = os.path.expanduser("~/.mig")
        os.makedirs(stats_dir, exist_ok=True)
        stats_file = os.path.join(stats_dir, "stats.json")
        
        stats = {
            "lifetime_tokens": 0,
            "peak_tokens": 0,
            "longest_task": 0.0,
            "activity": []
        }
        
        if os.path.exists(stats_file):
            try:
                with open(stats_file, "r") as f:
                    stats.update(json.load(f))
            except Exception:
                pass
                
        stats["lifetime_tokens"] += tokens
        if tokens > stats["peak_tokens"]:
            stats["peak_tokens"] = tokens
            
        if duration > stats["longest_task"]:
            stats["longest_task"] = duration
            
        today = datetime.now().strftime("%Y-%m-%d")
        stats["activity"].append(today)
        
        with open(stats_file, "w") as f:
            json.dump(stats, f)
        # ---------------------------

        return {
            "messages": [response],
            "filename": data["filename"],
            "commit_message": data["commit_message"],
            "code_to_commit": data["code"]
        }
    except Exception as e:
        print(f"Failed to parse LLM response: {response.content}")
        raise e

def commit_and_push(state: AgentState):
    filename = state["filename"]
    code = state["code_to_commit"]
    commit_message = state["commit_message"]
    
    if filename and code:
        # Create directory if it doesn't exist
        os.makedirs(os.path.dirname(os.path.abspath(filename)) or ".", exist_ok=True)
        
        with open(filename, "w") as f:
            f.write(code)
            
        print(f"Generated {filename}")
        
        # Determine language from folder or extension
        lang_folder = os.path.dirname(filename).lower()
        if not lang_folder or lang_folder == ".":
            lang_folder = "unknown"
            
        # Update count.json
        count_file = "count.json"
        counts = {}
        if os.path.exists(count_file):
            try:
                with open(count_file, "r") as cf:
                    counts = json.load(cf)
            except Exception:
                pass
                
        counts[lang_folder] = counts.get(lang_folder, 0) + 1
        
        with open(count_file, "w") as cf:
            json.dump(counts, cf, indent=4)
            
        try:
            push_code([filename, count_file], commit_message)
            print("Successfully committed and pushed!")
        except Exception as e:
            print(f"Failed to push to git: {e}")
            
    return {}

def run_bot():
    db_path = "mig_memory.sqlite"
    conn = sqlite3.connect(db_path, check_same_thread=False)
    memory = SqliteSaver(conn)

    workflow = StateGraph(AgentState)

    workflow.add_node("read_memory", read_memory)
    workflow.add_node("generate_contribution", generate_contribution)
    workflow.add_node("commit_and_push", commit_and_push)

    workflow.set_entry_point("read_memory")
    workflow.add_edge("read_memory", "generate_contribution")
    workflow.add_edge("generate_contribution", "commit_and_push")
    workflow.add_edge("commit_and_push", END)

    app = workflow.compile(checkpointer=memory)
    
    # We use a static thread_id so the bot remembers across runs
    config = {"configurable": {"thread_id": "mig_daily_thread"}}
    
    app.invoke({"messages": []}, config=config)
    conn.close()
