import "./static/style.scss";

import("./pkg/ellalang").then(module => {
    module.run_app();
});
