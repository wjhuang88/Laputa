import * as m from "./deploy/test_module.js"
import * as m2 from "./deploy/test_module_2.js"

//console.log(m.test)
var b = {test: m.test, testM: m, testM2: m2}
export default {b}