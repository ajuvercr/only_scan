var open = false;
function toggle_sidebar() {
    const sidebar = document.getElementById("sidebar");
    open = !open;

    if(open) {
        sidebar.classList.add("open");
    } else {
        sidebar.classList.remove("open");
    }
}
