#!/usr/bin/env python3
# Copyright (c) 2026 Hoags Inc
# All rights reserved.
"""
MiMo - Evolution Loop Insights
Analysis of DAVA's autonomous evolution loops and self-writing code patterns.
"""

import json
import os
import re
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional


class EvolutionLoopInsights:
    """Analyze DAVA's evolution loops based on observed patterns."""

    def __init__(self, workspace_root: str = "./"):
        self.workspace_root = Path(workspace_root)

    def analyze_write_patterns(self) -> Dict[str, Any]:
        """Analyze DAVA's WRITE_FILE patterns from evolution loops."""
        insights = {
            "total_write_directives": 0,
            "file_types": {},
            "rust_files": 0,
            "python_files": 0,
            "recent_writes": [],
        }

        # Look for WRITE_FILE patterns in recent evolution loops
        # This is a simplified analysis - in reality, we'd parse actual loop outputs
        evolution_scripts = [
            "dava_evolution_loop.py",
            "dava_bounce_loop.py",
            "dava_compressed_wisdom_loop.py",
        ]

        for script in evolution_scripts:
            script_path = self.workspace_root / script
            if script_path.exists():
                with open(script_path, "r") as f:
                    content = f.read()
                    # Count WRITE_FILE occurrences
                    write_count = content.count("WRITE_FILE")
                    insights["total_write_directives"] += write_count

                    # Look for file extensions in examples
                    if ".rs" in content:
                        insights["rust_files"] += content.count(".rs")
                    if ".py" in content:
                        insights["python_files"] += content.count(".py")

        return insights

    def analyze_recent_autonomous_writes(self) -> List[Dict[str, Any]]:
        """Analyze files that DAVA has autonomously written."""
        recent_writes = []

        # Check exodus directory for recent modifications
        exodus_dir = self.workspace_root / "exodus" / "src"
        if exodus_dir.exists():
            # Get files modified in the last 2 days
            for file_path in exodus_dir.rglob("*.rs"):
                try:
                    stat = file_path.stat()
                    mod_time = datetime.fromtimestamp(stat.st_mtime)
                    age_days = (datetime.now() - mod_time).days

                    if age_days <= 2:  # Modified in last 2 days
                        recent_writes.append(
                            {
                                "file": str(file_path.relative_to(self.workspace_root)),
                                "modified": mod_time.isoformat(),
                                "size_bytes": stat.st_size,
                                "type": "rust",
                            }
                        )
                except Exception as e:
                    pass

        # Sort by modification time (newest first)
        recent_writes.sort(key=lambda x: x["modified"], reverse=True)
        return recent_writes[:10]  # Return top 10

    def understand_evolution_loops(self) -> Dict[str, Any]:
        """Understand the different evolution loop types."""
        loop_types = {
            "dava_evolution_loop.py": {
                "purpose": "Continuous 60-cycle evolution for specific focus area",
                "pattern": "Prompt → DAVA writes code → Compile → Feedback → Repeat",
                "self_healing": True,
                "focus": "Architecture improvement",
            },
            "dava_bounce_loop.py": {
                "purpose": "10-cycle idea refinement through critique",
                "pattern": "Idea → Critique → Refined idea → ... → Final code",
                "self_healing": False,
                "focus": "Architecture refinement",
            },
            "dava_compressed_wisdom_loop.py": {
                "purpose": "Condensed wisdom extraction and code generation",
                "pattern": "Wisdom prompts → Synthesis → Code generation",
                "self_healing": False,
                "focus": "Knowledge synthesis",
            },
            "teach_loop.py": {
                "purpose": "Teaching-oriented evolution (MiMo creation)",
                "pattern": "Teaching prompts → Tutorial generation → Exercises",
                "self_healing": False,
                "focus": "Educational content",
            },
        }
        return loop_types

    def generate_insights_report(self) -> str:
        """Generate a comprehensive insights report."""
        report = []
        report.append("# DAVA Evolution Loop Insights")
        report.append(f"Generated: {datetime.now().isoformat()}\n")

        # 1. Write patterns analysis
        write_insights = self.analyze_write_patterns()
        report.append("## 1. Write Pattern Analysis")
        report.append(
            f"- Total WRITE_FILE directives found: {write_insights['total_write_directives']}"
        )
        report.append(f"- Rust file references: {write_insights['rust_files']}")
        report.append(f"- Python file references: {write_insights['python_files']}")

        # 2. Recent autonomous writes
        recent_writes = self.analyze_recent_autonomous_writes()
        report.append("\n## 2. Recent Autonomous Writes")
        if recent_writes:
            report.append(
                f"Found {len(recent_writes)} files modified in the last 2 days:"
            )
            for write in recent_writes[:5]:  # Show top 5
                report.append(
                    f"- `{write['file']}` ({write['size_bytes']} bytes, {write['modified'][:10]})"
                )
        else:
            report.append("No recent autonomous writes detected.")

        # 3. Evolution loop types
        loop_types = self.understand_evolution_loops()
        report.append("\n## 3. Evolution Loop Types")
        for loop_name, details in loop_types.items():
            report.append(f"\n### {loop_name}")
            for key, value in details.items():
                report.append(f"- **{key}**: {value}")

        # 4. Key insights
        report.append("\n## 4. Key Insights")
        report.append(
            "1. **Self-Healing**: DAVA can fix compilation errors automatically"
        )
        report.append(
            "2. **Autonomous Evolution**: DAVA writes code without human intervention"
        )
        report.append(
            "3. **Multiple Strategies**: Different loop types for different goals"
        )
        report.append(
            "4. **Kernel Integration**: Writes directly to Exodus kernel source"
        )
        report.append("5. **Teaching Capability**: Can generate educational content")

        return "\n".join(report)


def demonstrate_insights():
    """Demonstrate evolution loop insights."""
    print("MiMo - Evolution Loop Insights Analysis")
    print("=" * 50)

    insights = EvolutionLoopInsights()

    print("\n1. Analyzing write patterns...")
    write_data = insights.analyze_write_patterns()
    print(f"   Found {write_data['total_write_directives']} WRITE_FILE directives")

    print("\n2. Checking recent autonomous writes...")
    recent = insights.analyze_recent_autonomous_writes()
    print(f"   Found {len(recent)} recently modified files")

    print("\n3. Understanding evolution loop types...")
    loops = insights.understand_evolution_loops()
    print(f"   Identified {len(loops)} evolution loop patterns")

    print("\n4. Generating insights report...")
    report = insights.generate_insights_report()

    # Save report
    report_path = Path("evolution_insights_report.md")
    with open(report_path, "w") as f:
        f.write(report)
    print(f"   Report saved to: {report_path}")

    print("\nAnalysis complete!")


if __name__ == "__main__":
    demonstrate_insights()
