import os
import glob
import matplotlib.pyplot as plt


def count_lines(file_data):
    """Return a list of (filename, line_count) tuples, sorted descending."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    rs_files = glob.glob(os.path.join(script_dir, "*.rs"))

    if not rs_files:
        print("No .rs files found in:", script_dir)
        return None, script_dir

    file_data = []
    for filepath in rs_files:
        with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
            count = sum(1 for _ in f)
        file_data.append((os.path.basename(filepath), count))

    # Sort by line count, highest to lowest
    file_data.sort(key=lambda item: item[1], reverse=True)
    return file_data, script_dir


def draw_bar_chart(file_names, line_counts):
    fig, ax = plt.subplots(figsize=(max(8, len(file_names) * 1.2), 6))
    bars = ax.bar(file_names, line_counts, color="steelblue", edgecolor="black")

    for bar, count in zip(bars, line_counts):
        ax.text(
            bar.get_x() + bar.get_width() / 2,
            bar.get_height(),
            str(count),
            ha="center",
            va="bottom",
            fontsize=10,
        )

    ax.set_title("Line Count per .rs File")
    ax.set_xlabel("File")
    ax.set_ylabel("Number of Lines")
    plt.xticks(rotation=45, ha="right")
    plt.tight_layout()


def draw_pie_chart(file_names, line_counts):
    fig, ax = plt.subplots(figsize=(9, 9))

    def make_label(name, count):
        total = sum(line_counts)
        pct = (count / total) * 100
        return f"{name}\n{count} lines ({pct:.1f}%)"

    labels = [make_label(n, c) for n, c in zip(file_names, line_counts)]

    ax.pie(
        line_counts,
        labels=labels,
        autopct=None,
        startangle=90,
        counterclock=False,
        wedgeprops={"edgecolor": "white", "linewidth": 1},
    )
    ax.set_title("Line Count Share per .rs File")
    ax.axis("equal")  # keeps the pie circular
    plt.tight_layout()


def main():
    # Prompt for chart type
    print("Choose chart type:")
    print("1. Bar Chart")
    print("2. Pie Chart")
    choice = input("Enter 1 or 2: ").strip()

    if choice not in ("1", "2"):
        print("Invalid choice. Please enter 1 or 2.")
        return

    file_data, script_dir = count_lines(None)
    if not file_data:
        return

    file_names = [name for name, _ in file_data]
    line_counts = [count for _, count in file_data]

    if choice == "1":
        draw_bar_chart(file_names, line_counts)
        output_name = "rs_line_counts_bar.png"
    else:
        draw_pie_chart(file_names, line_counts)
        output_name = "rs_line_counts_pie.png"

    output_path = os.path.join(script_dir, output_name)
    plt.savefig(output_path, dpi=150)
    print(f"Graph saved to: {output_path}")
    plt.show()


if __name__ == "__main__":
    main()
