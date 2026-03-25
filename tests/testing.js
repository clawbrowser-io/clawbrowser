(() => {
  var failed = false;
  var errors = [];
  var observed_ids = {};

  function expectTrue(actual) {
    expectEqual(true, actual);
  }

  function expectFalse(actual) {
    expectEqual(false, actual);
  }

  function expectEqual(expected, actual) {
    if (_equal(expected, actual)) {
      _registerObservation('ok');
      return;
    }
    failed = true;
    _registerObservation('fail');
    var err = 'expected: ' + _displayValue(expected) + ', got: ' + _displayValue(actual) +
              '\n  script_id: ' + _currentScriptId();
    errors.push(err);
    console.error(err);
    throw new Error('expectEqual failed');
  }

  function fail(reason) {
    failed = true;
    _registerObservation('fail');
    errors.push(reason);
    console.error(reason);
    throw new Error('testing.fail()');
  }

  function expectError(expected, fn) {
    withError(function(err) {
      expectEqual(true, err.toString().includes(expected));
    }, fn);
  }

  function withError(cb, fn) {
    try {
      fn();
    } catch (err) {
      cb(err);
      return;
    }
    console.error('expected error but no error received\n');
    throw new Error('no error');
  }

  function assertOk() {
    if (failed) {
      document.title = 'FAIL: ' + errors.join('; ');
      return;
    }

    var script_ids = Object.keys(observed_ids);
    if (script_ids.length === 0) {
      document.title = 'FAIL: no test observations were recorded';
      return;
    }

    var scripts = document.getElementsByTagName('script');
    for (var i = 0; i < scripts.length; i++) {
      var script_id = scripts[i].id;
      if (!script_id) continue;

      var status = observed_ids[script_id];
      if (status !== 'ok') {
        document.title = "FAIL: script '" + script_id + "' " + (status || 'no assertions');
        return;
      }
    }

    document.title = 'PASS';
  }

  window.testing = {
    fail: fail,
    assertOk: assertOk,
    expectTrue: expectTrue,
    expectFalse: expectFalse,
    expectEqual: expectEqual,
    expectError: expectError,
    withError: withError,
  };

  window.$ = function(sel) { return document.querySelector(sel); };
  window.$$ = function(sel) { return document.querySelectorAll(sel); };

  function _equal(expected, actual) {
    if (expected === actual) return true;
    if (expected === null || actual === null) return false;
    if (expected === undefined || actual === undefined) return false;
    if (typeof expected !== typeof actual) return false;
    if (typeof expected !== 'object') return false;

    if (Array.isArray(expected)) {
      if (!Array.isArray(actual)) return false;
      if (expected.length !== actual.length) return false;
      for (var i = 0; i < expected.length; i++) {
        if (!_equal(expected[i], actual[i])) return false;
      }
      return true;
    }

    var keys = Object.keys(expected);
    if (keys.length !== Object.keys(actual).length) return false;
    for (var k = 0; k < keys.length; k++) {
      if (!_equal(expected[keys[k]], actual[keys[k]])) return false;
    }
    return true;
  }

  function _registerObservation(status) {
    var sid = _currentScriptId();
    if (!sid) return;
    if (observed_ids[sid] === 'fail') return;
    observed_ids[sid] = status;
  }

  function _currentScriptId() {
    var cs = document.currentScript;
    if (!cs) return null;
    return cs.id || null;
  }

  function _displayValue(value) {
    if (value === null) return 'null';
    if (value === undefined) return 'undefined';
    if (typeof value === 'string') return '"' + value + '"';
    if (typeof value === 'number' || typeof value === 'boolean') return String(value);
    if (Array.isArray(value)) return '[' + value.map(_displayValue).join(', ') + ']';
    try {
      var seen = [];
      return JSON.stringify(value, function(key, val) {
        if (val != null && typeof val == 'object') {
          if (seen.indexOf(val) >= 0) return;
          seen.push(val);
        }
        return val;
      });
    } catch(e) { return String(value); }
  }

  window.addEventListener('load', function() {
    testing.assertOk();
  });
})();
