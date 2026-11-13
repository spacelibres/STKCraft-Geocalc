import type { Metadata } from 'next';
import './style.css';
export const metadata: Metadata = { title:'STKCraft · GeoCalc', description:'STKCraft GeoCalc: explore space geometry and time.' };
export default function RootLayout({children}:{children:React.ReactNode}) {return <html lang="en"><body>{children}</body></html>;}
