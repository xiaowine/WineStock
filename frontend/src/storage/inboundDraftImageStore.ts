// 本文件拥有入库草稿本地图片的 IndexedDB 存取；它不保存普通表单字段或发起服务端上传。
const databaseName = "winestock-inbound-draft";
const storeName = "images";

/** 用当前草稿图片完整替换本地图片仓库，避免已删除字段留下 Blob。 */
export async function replaceInboundDraftImages(images: ReadonlyMap<string, File>): Promise<void> {
  const database = await openDatabase();
  await transactionDone(database, "readwrite", (store) => {
    store.clear();
    images.forEach((file, key) => store.put(file, key));
  });
}

/** 读取刷新前保存的本地图片文件。 */
export async function readInboundDraftImage(key: string): Promise<File | undefined> {
  const database = await openDatabase();
  return new Promise<File | undefined>((resolve, reject) => {
    const request = database.transaction(storeName, "readonly").objectStore(storeName).get(key);
    request.onsuccess = () => {
      database.close();
      resolve(request.result instanceof File ? request.result : undefined);
    };
    request.onerror = () => {
      database.close();
      reject(request.error);
    };
  });
}

/** 删除整张入库草稿拥有的本地图片。 */
export async function clearInboundDraftImages(): Promise<void> {
  const database = await openDatabase();
  await transactionDone(database, "readwrite", (store) => store.clear());
}

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(databaseName, 1);
    request.onupgradeneeded = () => request.result.createObjectStore(storeName);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

function transactionDone(
  database: IDBDatabase,
  mode: IDBTransactionMode,
  action: (store: IDBObjectStore) => void,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const transaction = database.transaction(storeName, mode);
    action(transaction.objectStore(storeName));
    transaction.oncomplete = () => {
      database.close();
      resolve();
    };
    transaction.onerror = () => {
      database.close();
      reject(transaction.error);
    };
    transaction.onabort = () => {
      database.close();
      reject(transaction.error);
    };
  });
}
