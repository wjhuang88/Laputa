import * as m from "./deploy/test_module.js"
import * as m2 from "./deploy/test_module_2.js"

let p1 = "<p>content: " + m.test + "</p>"
let p2 = "<p>content: " + m2.test + "</p>"
let body = "<!DOCTYPE html><html><body>" + p1 + p2 + "</body></html>"

export default {
    status: 200,
    headers: {
        "Content-type": "text/html",
        "Custom-Test": "From js test",
        "": ""
    },
    body
}