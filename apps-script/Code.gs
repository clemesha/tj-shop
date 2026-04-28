const SHEET_ID = '1V0d4ejGLPfGwOvKJzhjHxvC446m31R0YCeVnZQ9WNlk';
const WRITE_TOKEN = 'UXClpf3FxWOy9yyc9tY2R_NcbL9CkKweDecAacuz4RE';

function getSheet(name) {
  return SpreadsheetApp.openById(SHEET_ID).getSheetByName(name);
}

function sheetToJson(sheet) {
  const data = sheet.getDataRange().getValues();
  if (data.length < 2) return [];
  const headers = data[0];
  return data.slice(1).map(row => {
    const obj = {};
    headers.forEach((h, i) => obj[h] = row[i]);
    return obj;
  });
}

function jsonResponse(data) {
  return ContentService.createTextOutput(JSON.stringify(data))
    .setMimeType(ContentService.MimeType.JSON);
}

function doGet(e) {
  const action = e.parameter.action;

  if (action === 'getLists') {
    return jsonResponse(sheetToJson(getSheet('lists')));
  }

  if (action === 'getList') {
    const listId = e.parameter.listId || 'latest';
    const lists = sheetToJson(getSheet('lists'));
    let targetId = listId;
    if (listId === 'latest') {
      const active = lists.filter(l => l.status === 'active');
      if (active.length === 0) return jsonResponse({ error: 'No active list' });
      targetId = active[active.length - 1].list_id;
    }
    const items = sheetToJson(getSheet('list_items')).filter(i => String(i.list_id) === String(targetId));
    const list = lists.find(l => String(l.list_id) === String(targetId));
    return jsonResponse({ list, items });
  }

  if (action === 'getCustomProducts') {
    return jsonResponse(sheetToJson(getSheet('products_custom')));
  }

  return jsonResponse({ error: 'Unknown action' });
}

function doPost(e) {
  const body = JSON.parse(e.postData.contents);
  if (body.t !== WRITE_TOKEN) {
    return jsonResponse({ error: 'Unauthorized' });
  }
  const action = body.action;

  if (action === 'saveListItems') {
    const sheet = getSheet('list_items');
    const listId = body.listId;
    const items = body.items; // [{ sku, quantity }]

    // Remove existing items for this list
    const data = sheet.getDataRange().getValues();
    for (let i = data.length - 1; i >= 1; i--) {
      if (String(data[i][0]) === String(listId)) sheet.deleteRow(i + 1);
    }

    // Add new items as plain text to prevent auto-formatting
    if (items.length > 0) {
      const lastRow = sheet.getLastRow();
      const range = sheet.getRange(lastRow + 1, 1, items.length, 4);
      range.setNumberFormat('@');
      range.setValues(items.map(item => [listId, String(item.sku), String(item.quantity), 'FALSE']));
    }

    return jsonResponse({ ok: true });
  }

  if (action === 'removeListItem') {
    const sheet = getSheet('list_items');
    const data = sheet.getDataRange().getValues();
    for (let i = data.length - 1; i >= 1; i--) {
      if (String(data[i][0]) === String(body.listId) && String(data[i][1]) === String(body.sku)) {
        sheet.deleteRow(i + 1);
      }
    }
    return jsonResponse({ ok: true });
  }

  if (action === 'toggleChecked') {
    const sheet = getSheet('list_items');
    const data = sheet.getDataRange().getValues();
    for (let i = 1; i < data.length; i++) {
      if (data[i][0] === body.listId && data[i][1] === body.sku) {
        const current = data[i][3];
        sheet.getRange(i + 1, 4).setValue(current === true || current === 'TRUE' ? false : true);
        return jsonResponse({ ok: true, checked: !(current === true || current === 'TRUE') });
      }
    }
    return jsonResponse({ error: 'Item not found' });
  }

  if (action === 'addCustomProduct') {
    const sheet = getSheet('products_custom');
    const p = body.product;
    sheet.appendRow([p.sku, p.name, p.price, p.size, p.category, p.image_url || '']);
    return jsonResponse({ ok: true });
  }

  if (action === 'editCustomProduct') {
    const sheet = getSheet('products_custom');
    const data = sheet.getDataRange().getValues();
    for (let i = 1; i < data.length; i++) {
      if (data[i][0] === body.sku) {
        const p = body.product;
        sheet.getRange(i + 1, 1, 1, 6).setValues([[p.sku, p.name, p.price, p.size, p.category, p.image_url || '']]);
        return jsonResponse({ ok: true });
      }
    }
    return jsonResponse({ error: 'Product not found' });
  }

  if (action === 'deleteCustomProduct') {
    const sheet = getSheet('products_custom');
    const data = sheet.getDataRange().getValues();
    for (let i = 1; i < data.length; i++) {
      if (data[i][0] === body.sku) {
        sheet.deleteRow(i + 1);
        return jsonResponse({ ok: true });
      }
    }
    return jsonResponse({ error: 'Product not found' });
  }

  if (action === 'createList') {
    const sheet = getSheet('lists');
    sheet.appendRow([body.listId, body.label, new Date().toISOString().slice(0, 10), body.status || 'active']);
    return jsonResponse({ ok: true });
  }

  return jsonResponse({ error: 'Unknown action' });
}
