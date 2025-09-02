import { fetchApplications } from "./utils/authentik";

document.addEventListener('DOMContentLoaded', function () {
    const appList = document.getElementById('app-list')!;
    // const launchButton = document.getElementById('launch-button')!;
    const refreshButton = document.getElementById('refresh-button')!;

    // // Handle application selection
    // appList.addEventListener('click', function (event) {
    //     if (event.target.tagName === 'LI') {
    //         const previouslySelected = appList.querySelector('li.selected');
    //         if (previouslySelected) {
    //             previouslySelected.classList.remove('selected');
    //         }
    //         event.target.classList.add('selected');
    //     }
    // });

    // Initialize the popup
    fetchApplications().then(apps => {
        apps.filter(app => app.launchUrl).forEach(app => {
            const listItem = document.createElement('li');
            listItem.textContent = app.name;
            listItem.dataset.url = app.launchUrl!;
            appList!.appendChild(listItem);
        });
    }).catch(error => {
        console.error('Error fetching applications:', error);
    });
});
