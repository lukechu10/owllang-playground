import "./static/style.scss";

import("./pkg/owllang").then(module => {
    module.run_app();
});
