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
        input_tokens = 0
        output_tokens = 0
        total_tokens = 0
        
        if hasattr(response, 'usage_metadata') and response.usage_metadata:
            input_tokens = response.usage_metadata.get('input_tokens', 0)
            output_tokens = response.usage_metadata.get('output_tokens', 0)
            total_tokens = response.usage_metadata.get('total_tokens', 0)
        
        if total_tokens == 0:
            total_tokens = (len(str(messages)) + len(content)) // 4
            input_tokens = total_tokens // 2
            output_tokens = total_tokens // 2
            
        model_name = "unknown"
        if hasattr(response, 'response_metadata') and response.response_metadata:
            model_name = response.response_metadata.get('model_name', "unknown")
            
        # Calculate approximate cost based on Groq pricing (per 1M tokens)
        prices = {
            "llama3-8b-8192": (0.05, 0.08),
            "llama3-70b-8192": (0.59, 0.79),
            "mixtral-8x7b-32768": (0.24, 0.24),
            "gemma-7b-it": (0.07, 0.07),
        }
        
        # default to 70b price if unknown
        price_in, price_out = prices.get(model_name.lower(), (0.59, 0.79))
        cost = (input_tokens / 1_000_000) * price_in + (output_tokens / 1_000_000) * price_out
            
        stats_dir = os.path.expanduser("~/.mig")
        os.makedirs(stats_dir, exist_ok=True)
        stats_file = os.path.join(stats_dir, "stats.json")
        analytics_file = os.path.join(stats_dir, "groq_analytics.json")
        
        # 1. Update basic stats.json
        stats = {"lifetime_tokens": 0, "peak_tokens": 0, "longest_task": 0.0, "activity": []}
        if os.path.exists(stats_file):
            try:
                with open(stats_file, "r") as f: stats.update(json.load(f))
            except Exception: pass
                
        stats["lifetime_tokens"] += total_tokens
        if total_tokens > stats["peak_tokens"]: stats["peak_tokens"] = total_tokens
        if duration > stats["longest_task"]: stats["longest_task"] = duration
        today = datetime.now().strftime("%Y-%m-%d")
        stats["activity"].append(today)
        with open(stats_file, "w") as f: json.dump(stats, f)
        
        # 2. Update groq_analytics.json
        groq_stats = {"total_cost": 0.0, "models": {}, "daily": {}}
        if os.path.exists(analytics_file):
            try:
                with open(analytics_file, "r") as f: groq_stats.update(json.load(f))
            except Exception: pass
            
        groq_stats["total_cost"] += cost
        
        # Update per model stats
        if model_name not in groq_stats["models"]:
            groq_stats["models"][model_name] = {"total_tokens": 0, "cost": 0.0, "runs": 0}
        groq_stats["models"][model_name]["total_tokens"] += total_tokens
        groq_stats["models"][model_name]["cost"] += cost
        groq_stats["models"][model_name]["runs"] += 1
        
        # Update daily stats
        if today not in groq_stats["daily"]:
            groq_stats["daily"][today] = {"tokens": 0, "cost": 0.0}
        groq_stats["daily"][today]["tokens"] += total_tokens
        groq_stats["daily"][today]["cost"] += cost
        
        with open(analytics_file, "w") as f: json.dump(groq_stats, f)
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
