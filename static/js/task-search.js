const taskSearch = document.getElementById("task-search");
const taskList = document.getElementById("task-list");
const taskSearchEmpty = document.getElementById("task-search-empty");

if (taskSearch && taskList && taskSearchEmpty) {
    const tasks = Array.from(taskList.rows, row => ({
        row,
        fields: Array.from(row.querySelectorAll("[data-searchable]"), cell =>
            cell.textContent.trim().toLowerCase())
    }));

    function filterTasks() {
        const query = taskSearch.value.trim().toLowerCase();
        let visibleCount = 0;

        for (const { row, fields } of tasks) {
            const matches = fields.some(text => text.includes(query));
            row.hidden = !matches;
            if (matches) visibleCount++;
        }

        taskSearchEmpty.hidden = query === "" || visibleCount > 0;
    }

    taskSearch.addEventListener("input", filterTasks);
    filterTasks();
}
