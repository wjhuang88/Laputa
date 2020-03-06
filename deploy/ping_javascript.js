import * as m from "./deploy/test_module.js"
import * as m2 from "./deploy/test_module_2.js"

let uri = request.uri
let query = request.query
let testHeader = request.header("User-Agent")

let p1 = "<p>content: " + m.test + "</p>"
let p2 = "<p>content: " + m2.test + "</p>"
let p3 = "<p>route uri: " + uri + "</p>"
let p4 = "<p>route query: " + query + "</p>"
let p5 = "<p>" + testHeader + "</p>"
let body = "<!DOCTYPE html><html><body>" + p1 + p2 + p3 + p4 + p5 + "</body></html>"

export default {
    status: 200,
    headers: {
        "Content-type": "text/html",
        "Custom-Test": "From js test",
    },
    body
}