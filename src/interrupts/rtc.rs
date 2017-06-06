use port_io::Port;
use core::sync::atomic::{AtomicUsize, Ordering};
pub use irq_safety::{disable_interrupts, enable_interrupts, interrupts_enabled};
use interrupts::rtc;
use spin::Mutex;

//standard port to write to on CMOS to select registers
const CMOS_WRITE_PORT: u16 = 0x70;
//standard port to read register values from on CMOS or write to to change settings
const CMOS_READ_PORT: u16 = 0x71;


pub static RTC_TICKS: AtomicUsize = AtomicUsize::new(0);
//used to select register
static CMOS_WRITE: Mutex<Port<u8>> = Mutex::new( Port::new(CMOS_WRITE_PORT));
//used to change cmos settings
static CMOS_WRITE_SETTINGS: Mutex<Port<u8>> = Mutex::new(Port::new(CMOS_READ_PORT));
//used to read from cmos register
static CMOS_READ: Mutex<Port<u8>> = Mutex::new( Port::new(CMOS_READ_PORT));


//write a u8 to the CMOS port (0x70)
fn write_cmos(value: u8){

    unsafe{CMOS_WRITE.lock().write(value)}

}


//read a u8 from CMOS port 0x71
fn read_cmos()->u8{
    
    CMOS_READ.lock().read()
    
}



//returns true if update in progress, false otherwise
fn get_update_in_progress()-> bool{
    
    //writing to this register causes cmos to output 1 if rtc update in progress 
    write_cmos(0x0A);
    let is_in_progress: bool = read_cmos() == 1;
    is_in_progress

}


//register value is entered, rtc's associated value is output, waits for update in progress signal to end
fn read_register(register: u8)->u8{
    
    //waits for "update in progress" signal to finish in order to read correct values
    while get_update_in_progress() {}
    write_cmos(register);

    //converts bcd value to binary value which is what is used for printing 
    let bcd = read_cmos();
    
    (bcd/16)*10 + (bcd & 0xf)


}

pub struct time{
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u8,
    months: u8,
    years: u8,

}

//call this function to print RTC's date and time
pub fn read_rtc()->time{

    //calls read register function which writes to port 0x70 to set RTC then reads from 0x71 which outputs correct value
    let second = read_register(0x00);
    let minute = read_register(0x02);
    let hour = read_register(0x04);
    let day = read_register(0x07);
    let month = read_register(0x08);
    let year = read_register(0x09);

    
    trace!("Time - {}:{}:{} {}/{}/{}", hour, minute,second, month, day, year);

    time{seconds:second, minutes: minute, hours: hour, days: day, months: month, years: year}

}


//turn on IRQ 8, rtc begins sending interrupts 
pub fn enable_rtc_interrupt()
{
    disable_interrupts();
    write_cmos(0x0C);
    read_cmos();
    //select cmos register 0x8B
    write_cmos(0x8B);

    //value needed to turn on bit 6 of register B
    let prev = read_cmos();

    //we want it to go back to register 0x8B, it was reset when read
    write_cmos(0x8B);

    //here we don't use the cmos_write function because that only writes to port 0x70, in this case we need to write to 0x71
    //writing to 0x71 because not selecting register, setting rtc
    
    unsafe{CMOS_WRITE_SETTINGS.lock().write(prev | 0x40)};

    
    enable_interrupts();

    trace!("RTC Enabled!");

}


/// the heartbeatperiod in milliseconds
const heartbeat_period_ms: u64 = 1000;

//used to change periodic interrupt rate of RTC, ranges from 3 to 15, 3 is 8khz 15 is 2 HZ
pub fn change_rtc_frequency(rate: u8){

    disable_interrupts();
    
    //bottom 4 bits of register A are rate, setting them to rate we want without altering top 4 bits
    write_cmos(0x8A);
    let prev = read_cmos();
    write_cmos(0x8A); 

    unsafe{CMOS_WRITE_SETTINGS.lock().write(((prev & 0xF0)|rate))};

    enable_interrupts();
    trace!("rtc rate frequency changed!");
}


//counts interrupts from RTC
pub fn handle_rtc_interrupt() {
    
    write_cmos(0x0C);
    read_cmos();
    let old_tick = TICKS.fetch_add(1,Ordering::SeqCst);
    let rtc_ticks = old_tick +1;
  
    
    if (rtc_ticks % 128) == 0 {
        trace!("[rtc heartbeat] {} seconds have passed (rtc ticks={})", heartbeat_period_ms/1000, rtc_ticks);
    }


}